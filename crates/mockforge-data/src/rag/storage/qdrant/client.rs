use super::*;

impl QdrantStorage {
    pub(super) fn endpoint(&self, suffix: &[&str]) -> url::Url {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .expect("validated HTTP URL")
            .pop_if_empty()
            .extend(["collections", self.collection.as_str()])
            .extend(suffix.iter().copied());
        url
    }

    pub(super) fn request(&self, method: Method, suffix: &[&str]) -> reqwest::RequestBuilder {
        self.client.request(method, self.endpoint(suffix))
    }

    pub(super) async fn send(request: reqwest::RequestBuilder) -> Result<Response> {
        let response = request.send().await.map_err(|_| Error::generic("Qdrant request failed"))?;
        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Qdrant returned HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(response)
    }

    pub(super) async fn result(request: reqwest::RequestBuilder) -> Result<Value> {
        Self::response_result(Self::send(request).await?).await
    }

    pub(super) async fn response_result(mut response: Response) -> Result<Value> {
        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Qdrant returned HTTP {}",
                response.status().as_u16()
            )));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::generic("Qdrant response interrupted"))?
        {
            if bytes.len().saturating_add(chunk.len()) > 64 * 1024 * 1024 {
                return Err(Error::generic("Qdrant JSON response exceeds 64 MiB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let mut body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Error::generic("Invalid Qdrant JSON response"))?;
        body.get_mut("result")
            .map(Value::take)
            .ok_or_else(|| Error::generic("Qdrant response omitted result"))
    }
}
