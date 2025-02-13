use bollard::{image::BuildImageOptions, Docker};
use futures::TryStreamExt;
use tracing::debug;

pub(crate) mod netstat;

const INSPECTOR_CONTAINER_DOCKERFILE: &str = r#"
FROM alpine:3.17
RUN apk add --no-cache iproute2 net-tools
CMD ["sh", "-c", "netstat -tupln | grep / | grep -v 'Proto'"]
"#;

/// Build a network inspector image from the above raw Dockerfile.
///
/// Returns the name/tag of the built image on success.
pub async fn build_sidecar_image(docker: &Docker) -> Result<String, DockerSidecarError> {
    let image_name = "ivynet_network_inspector:0.1.0";

    let build_opts = BuildImageOptions {
        t: image_name,
        rm: true,      // Remove intermediate containers
        forcerm: true, // Always remove intermediate containers
        ..Default::default()
    };

    let mut archive_data = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut archive_data);
        let mut header = tar::Header::new_gnu();
        header.set_size(INSPECTOR_CONTAINER_DOCKERFILE.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        builder.append_data(
            &mut header,
            "Dockerfile",
            INSPECTOR_CONTAINER_DOCKERFILE.as_bytes(),
        )?;
        builder.finish()?;
    }

    let mut build_stream = docker.build_image(build_opts, None, Some(archive_data.into()));

    // Stream the build output to stdout (optional, can be omitted or logged)
    while let Some(chunk_result) = build_stream.try_next().await? {
        if let Some(output) = chunk_result.stream {
            debug!("{}", output);
        }
        if let Some(error) = chunk_result.error {
            eprintln!("{}", error);
        }
    }

    Ok(image_name.to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum DockerSidecarError {
    #[error("Docker error: {0}")]
    DockerError(#[from] bollard::errors::Error),
    #[error("Builder error: {0}")]
    BuilderError(#[from] std::io::Error),
}

#[cfg(test)]
mod inspector_container_tests {
    use super::*;
    use bollard::Docker;

    #[tokio::test]
    async fn test_build_sidecar_image() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let image_name = build_sidecar_image(&docker).await.unwrap();
        assert_eq!(image_name, "ivynet_network_inspector:0.1.0");
    }
}
