// Exact native source excerpts from adapters/mod.rs, not replacement algorithms.
fn execute_http_request_bytes(request: &AdapterHttpRequest) -> Result<Vec<u8>, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::new();
    let mut builder = match request.method {
        AdapterHttpMethod::Get => client.get(&request.url),
        AdapterHttpMethod::Post => client.post(&request.url),
    };
    if let Some(content_type) = request.content_type {
        builder = builder.header("Content-Type", content_type);
    }
    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }
    let response = builder.send().map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "{} API returned error: {}",
            request.service_label,
            response.status()
        ));
    }
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}

impl DataAdapter for DemAdapter {
    /// Registered adapter identifier (`self.id`).
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    /// Build the Terrarium tile request via the inherent helper.
    fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        DemAdapter::build_fetch_request(self, bbox, time_range, registry)
    }

    /// DEM responses are binary — the body must be consumed.
    fn needs_fetch_body(&self) -> bool {
        true
    }

    /// Binary PNG terrain can't go through the text pipeline.
    fn requires_specialized_fetch_path(&self) -> bool {
        true
    }

    /// Fail-closed: Terrarium responses are binary PNG; the typed DEM
    /// fetch methods decode them.
    fn handle_fetch_body(&self, _body: &str) -> Result<(), String> {
        Err(
            "Terrarium responses are binary PNG; call fetch_region_dem or fetch_region_dem_async"
                .into(),
        )
    }

    /// Native: delegates to `fetch_region_dem` and discards the asset.
    /// wasm32: fail-closed, directs callers to `fetch_region_dem_async`.
    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.fetch_region_dem(bbox, registry).map(|_| ())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (bbox, registry);
            Err("use fetch_region_dem_async on wasm32 to fetch and compile terrain".into())
        }
    }

    /// Native: `fetch_region_dem` compiled to NQuins. wasm32:
    /// fail-closed, directs callers to `fetch_region_dem_async`.
    fn fetch_region_features(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<crate::NQuin>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.fetch_region_dem(bbox, registry)
                .map(|asset| asset.to_quins())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (bbox, registry);
            Err("use fetch_region_dem_async on wasm32 to receive the compiled terrain asset".into())
        }
    }

    /// Fail-closed: binary PNG cannot be parsed as text.
    fn parse_response(&self, _body: &str) -> Result<Vec<crate::NQuin>, String> {
        Err("Terrarium responses are binary PNG; call the typed DEM fetch method".into())
    }

    /// Endpoint URL with the `{z}` tile template truncated.
    fn primary_endpoint(&self) -> &str {
        self.endpoint.split("{z}").next().unwrap_or(&self.endpoint)
    }

    /// Tile estimate via the inherent helper.
    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        DemAdapter::estimate_tile_count(self, bbox)
    }
}

