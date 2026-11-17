/* Webizen WebGL2 probe. Loaded before WASM.
   Hot path after the sample cap: integer counters only, no heap, no getError.
   First N uploads/draws capture UNPACK/PBO/draw params for diagnosis.
   Read window.__webizenGlDebug.snapshot() after the scene has presented. */
(function installWebizenGlDebug() {
  if (window.__webizenGlDebug && window.__webizenGlDebug.installed) return;

  var UPLOAD_CAP = 64;
  var DRAW_CAP = 32;
  var ERROR_CAP = 48;
  var ANGLE_CAP = 16;
  var GL = {
    NO_ERROR: 0,
    INVALID_OPERATION: 0x0502,
    PIXEL_UNPACK_BUFFER: 0x88EC,
    PIXEL_UNPACK_BUFFER_BINDING: 0x88EF,
    UNPACK_ALIGNMENT: 0x0CF5,
    UNPACK_ROW_LENGTH: 0x0CF2,
    UNPACK_SKIP_PIXELS: 0x0CF4,
    UNPACK_SKIP_ROWS: 0x0CF3,
    UNPACK_IMAGE_HEIGHT: 0x806E,
    UNPACK_SKIP_IMAGES: 0x806D,
    BUFFER_SIZE: 0x8764,
    RGBA_INTEGER: 0x8D99,
    UNSIGNED_INT: 0x1405
  };

  var state = {
    installed: true,
    contexts: 0,
    texSubImage2d: 0,
    texSubImage3d: 0,
    texImage2d: 0,
    compressedTexSubImage2d: 0,
    drawArrays: 0,
    drawElements: 0,
    drawArraysInstanced: 0,
    drawElementsInstanced: 0,
    drawArraysVerts: 0,
    drawElementsCount: 0,
    drawInstancedInstances: 0,
    skyLikeDrawArrays3: 0,
    invalidOperation: 0,
    otherErrors: 0,
    angleOverflow: 0,
    pboBoundUploads: 0,
    clientMemoryUploads: 0,
    integerRgbaUploads: 0,
    uploads: [],
    draws: [],
    errors: [],
    angle: []
  };

  function errName(code) {
    if (code === 0) return 'NO_ERROR';
    if (code === 0x0502) return 'INVALID_OPERATION';
    if (code === 0x0500) return 'INVALID_ENUM';
    if (code === 0x0501) return 'INVALID_VALUE';
    if (code === 0x0505) return 'OUT_OF_MEMORY';
    return '0x' + (code >>> 0).toString(16);
  }

  function enumName(value) {
    if (value === 0x1908) return 'RGBA';
    if (value === GL.RGBA_INTEGER) return 'RGBA_INTEGER';
    if (value === 0x1401) return 'UNSIGNED_BYTE';
    if (value === GL.UNSIGNED_INT) return 'UNSIGNED_INT';
    if (value === 0x0DE1) return 'TEXTURE_2D';
    return value;
  }

  function srcKind(value) {
    if (value == null) return 'null';
    if (typeof value === 'number') return 'pbo_offset';
    if (typeof value === 'object') {
      if (typeof value.byteLength === 'number') return 'array_buffer_view';
      if (typeof value.width === 'number') return 'image_source';
    }
    return typeof value;
  }

  function peekError(gl, where, extra) {
    if (state.errors.length >= ERROR_CAP) return;
    var code = gl.getError();
    if (code === GL.NO_ERROR) return;
    if (code === GL.INVALID_OPERATION) state.invalidOperation += 1;
    else state.otherErrors += 1;
    state.errors.push({ where: where, error: errName(code), extra: extra });
    return errName(code);
  }

  function recordUpload(kind, gl, args) {
    var sampling = state.uploads.length < UPLOAD_CAP;
    var a = args;
    var width = null, height = null, format = null, type = null, src = null, offset = null;
    if (kind === 'texSubImage2D') {
      if (a.length >= 9) {
        width = a[4]; height = a[5]; format = a[6]; type = a[7]; src = a[8];
        offset = typeof a[9] === 'number' ? a[9] : (typeof src === 'number' ? src : null);
      } else if (a.length >= 7) {
        format = a[4]; type = a[5]; src = a[6];
      }
    } else if (kind === 'texSubImage3D' && a.length >= 11) {
      width = a[5]; height = a[6]; format = a[8]; type = a[9]; src = a[10];
    } else if (kind === 'texImage2D' && a.length >= 9) {
      width = a[3]; height = a[4]; format = a[6]; type = a[7]; src = a[8];
    }
    var kindSrc = srcKind(src);
    var pbo = false;
    if (sampling) {
      try { pbo = !!gl.getParameter(GL.PIXEL_UNPACK_BUFFER_BINDING); }
      catch (e) { pbo = kindSrc === 'pbo_offset'; }
    } else {
      pbo = kindSrc === 'pbo_offset';
    }
    if (kindSrc === 'pbo_offset' || pbo) state.pboBoundUploads += 1;
    else state.clientMemoryUploads += 1;
    if (format === GL.RGBA_INTEGER) state.integerRgbaUploads += 1;
    if (!sampling) return;
    var unpack = {
      unpackAlignment: gl.getParameter(GL.UNPACK_ALIGNMENT),
      unpackRowLength: gl.getParameter(GL.UNPACK_ROW_LENGTH),
      unpackSkipPixels: gl.getParameter(GL.UNPACK_SKIP_PIXELS),
      unpackSkipRows: gl.getParameter(GL.UNPACK_SKIP_ROWS),
      unpackImageHeight: gl.getParameter(GL.UNPACK_IMAGE_HEIGHT),
      unpackSkipImages: gl.getParameter(GL.UNPACK_SKIP_IMAGES),
      pboBound: pbo,
      pboSize: null
    };
    if (pbo) {
      try { unpack.pboSize = gl.getBufferParameter(GL.PIXEL_UNPACK_BUFFER, GL.BUFFER_SIZE); }
      catch (e) { unpack.pboSize = 'unreadable'; }
    }
    var entry = {
      n: state.texSubImage2d + state.texSubImage3d + state.texImage2d,
      kind: kind,
      width: width,
      height: height,
      format: enumName(format),
      type: enumName(type),
      src: kindSrc,
      srcBytes: src && typeof src.byteLength === 'number' ? src.byteLength : null,
      srcOffset: offset,
      unpack: unpack
    };
    var err = peekError(gl, kind, entry);
    if (err) entry.error = err;
    state.uploads.push(entry);
  }

  function recordDraw(kind, gl, args) {
    var count = 0, instances = 1;
    if (kind === 'drawArrays') {
      count = args[2] || 0;
      if (count === 3) state.skyLikeDrawArrays3 += 1;
      state.drawArraysVerts += count;
    } else if (kind === 'drawElements') {
      count = args[1] || 0;
      state.drawElementsCount += count;
    } else if (kind === 'drawArraysInstanced') {
      count = args[2] || 0;
      instances = args[3] || 0;
      state.drawInstancedInstances += instances;
    } else if (kind === 'drawElementsInstanced') {
      // drawElementsInstanced(mode, count, type, offset, instanceCount)
      count = args[1] || 0;
      instances = args[4] || 0;
      state.drawElementsCount += count;
      state.drawInstancedInstances += instances;
    }
    if (state.draws.length >= DRAW_CAP) return;
    var entry = { kind: kind, count: count, instances: instances };
    peekError(gl, kind, entry);
    state.draws.push(entry);
  }

  function wrap(gl, name, after) {
    var original = gl[name];
    if (typeof original !== 'function') return;
    gl[name] = function () {
      var result = original.apply(this, arguments);
      after(arguments);
      return result;
    };
  }

  function instrument(gl) {
    if (!gl || gl.__webizenGlDebug) return gl;
    gl.__webizenGlDebug = true;
    state.contexts += 1;
    wrap(gl, 'texSubImage2D', function (args) {
      state.texSubImage2d += 1;
      recordUpload('texSubImage2D', gl, args);
    });
    wrap(gl, 'texSubImage3D', function (args) {
      state.texSubImage3d += 1;
      recordUpload('texSubImage3D', gl, args);
    });
    wrap(gl, 'texImage2D', function (args) {
      state.texImage2d += 1;
      recordUpload('texImage2D', gl, args);
    });
    wrap(gl, 'compressedTexSubImage2D', function () {
      state.compressedTexSubImage2d += 1;
    });
    wrap(gl, 'drawArrays', function (args) {
      state.drawArrays += 1;
      recordDraw('drawArrays', gl, args);
    });
    wrap(gl, 'drawElements', function (args) {
      state.drawElements += 1;
      recordDraw('drawElements', gl, args);
    });
    wrap(gl, 'drawArraysInstanced', function (args) {
      state.drawArraysInstanced += 1;
      recordDraw('drawArraysInstanced', gl, args);
    });
    wrap(gl, 'drawElementsInstanced', function (args) {
      state.drawElementsInstanced += 1;
      recordDraw('drawElementsInstanced', gl, args);
    });
    return gl;
  }

  var originalGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, attrs) {
    var ctx = originalGetContext.call(this, type, attrs);
    if (ctx && String(type).toLowerCase().indexOf('webgl2') !== -1) instrument(ctx);
    return ctx;
  };

  var origWarn = console.warn.bind(console);
  var origError = console.error.bind(console);
  function interceptAngle(args) {
    var text = '';
    for (var i = 0; i < args.length; i++) text += String(args[i]);
    if (text.indexOf('glTexSubImage2DRobustANGLE') === -1 && text.indexOf('parameters overflow') === -1) return;
    state.angleOverflow += 1;
    if (state.angle.length < ANGLE_CAP) {
      state.angle.push({
        n: state.angleOverflow,
        lastUpload: state.uploads.length ? state.uploads[state.uploads.length - 1] : null
      });
    }
  }
  console.warn = function () { interceptAngle(arguments); return origWarn.apply(console, arguments); };
  console.error = function () { interceptAngle(arguments); return origError.apply(console, arguments); };

  function snapshot() {
    var failed = [];
    var pboFailed = [];
    var clientFailed = [];
    var integerFailed = [];
    for (var i = 0; i < state.uploads.length; i++) {
      var u = state.uploads[i];
      if (!u.error) continue;
      failed.push(u);
      if (u.src === 'pbo_offset' || (u.unpack && u.unpack.pboBound)) pboFailed.push(u);
      if (u.src === 'array_buffer_view') clientFailed.push(u);
      if (u.format === 'RGBA_INTEGER') integerFailed.push(u);
    }
    return {
      schema: 'webizen.gl-debug/v1',
      contexts: state.contexts,
      uploads: {
        texSubImage2d: state.texSubImage2d,
        texSubImage3d: state.texSubImage3d,
        texImage2d: state.texImage2d,
        compressedTexSubImage2d: state.compressedTexSubImage2d,
        pboBound: state.pboBoundUploads,
        clientMemory: state.clientMemoryUploads,
        integerRgba: state.integerRgbaUploads
      },
      draws: {
        drawArrays: state.drawArrays,
        drawElements: state.drawElements,
        drawArraysInstanced: state.drawArraysInstanced,
        drawElementsInstanced: state.drawElementsInstanced,
        skyLikeDrawArrays3: state.skyLikeDrawArrays3,
        drawArraysVerts: state.drawArraysVerts,
        drawElementsCount: state.drawElementsCount,
        drawInstancedInstances: state.drawInstancedInstances,
        first: state.draws
      },
      errors: {
        invalidOperation: state.invalidOperation,
        other: state.otherErrors,
        angleOverflow: state.angleOverflow,
        first: state.errors,
        angle: state.angle
      },
      failedUploads: failed,
      failedPbo: pboFailed,
      failedClient: clientFailed,
      failedIntegerRgba: integerFailed,
      firstUploads: state.uploads,
      meshLikelyDrawn: (state.drawElements + state.drawElementsInstanced) > 0
    };
  }

  function paintOverlay() {
    if (String(location.search || '').indexOf('gl-debug=1') === -1) return;
    var el = document.getElementById('webizen-gl-debug');
    if (!el) {
      el = document.createElement('pre');
      el.id = 'webizen-gl-debug';
      el.setAttribute('role', 'status');
      document.body.appendChild(el);
    }
    var s = snapshot();
    el.textContent =
      'GL debug  IO=' + s.errors.invalidOperation +
      ' overflow=' + s.errors.angleOverflow +
      ' texSub=' + s.uploads.texSubImage2d +
      ' pbo=' + s.uploads.pboBound +
      ' client=' + s.uploads.clientMemory +
      ' rgba32ui=' + s.uploads.integerRgba +
      '\ndrawArrays=' + s.draws.drawArrays +
      ' drawElements=' + s.draws.drawElements +
      ' instanced=' + s.draws.drawElementsInstanced +
      ' sky3=' + s.draws.skyLikeDrawArrays3 +
      ' meshDrawn=' + s.meshLikelyDrawn +
      '\nfailedPbo=' + s.failedPbo.length +
      ' failedClient=' + s.failedClient.length +
      ' failedRGBA32UI=' + s.failedIntegerRgba.length;
  }

  window.__webizenGlDebug = {
    installed: true,
    snapshot: snapshot,
    overlay: paintOverlay
  };
  if (String(location.search || '').indexOf('gl-debug=1') !== -1) {
    setInterval(paintOverlay, 1000);
  }
})();
