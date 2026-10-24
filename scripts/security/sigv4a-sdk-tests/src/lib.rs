#[cfg(test)]
mod tests {
    use aws_sdk_s3::{
        config::{BehaviorVersion, Credentials, Region},
        presigning::PresigningConfig,
        Client,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn real_sdk_mrap_presigning_keeps_sigv4a_and_session_token() {
        let configuration = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "AKIDEXAMPLE",
                "public-test-secret",
                Some("public-test-session".into()),
                None,
                "offline-test",
            ))
            .build();
        let signed = Client::from_conf(configuration)
            .get_object()
            .bucket("arn:aws:s3::123456789012:accesspoint/mfzwi23gnjvgw.mrap")
            .key("object")
            .presigned(PresigningConfig::expires_in(Duration::from_secs(900)).unwrap())
            .await
            .expect("real SDK MRAP signing must succeed without network or live credentials");
        let uri = signed.uri();
        assert!(
            uri.starts_with(
                "https://mfzwi23gnjvgw.mrap.accesspoint.s3-global.amazonaws.com/object?"
            ),
            "{uri}"
        );
        assert!(uri.contains("X-Amz-Algorithm=AWS4-ECDSA-P256-SHA256"), "{uri}");
        assert!(uri.contains("X-Amz-Region-Set=%2A"), "{uri}");
        assert!(uri.contains("X-Amz-Security-Token=public-test-session"), "{uri}");
        assert!(uri.contains("%2Fs3%2Faws4_request"), "{uri}");
        assert!(!uri.contains("%2Fus-east-1%2Fs3"), "MRAP scope must be regionless: {uri}");
    }
}
