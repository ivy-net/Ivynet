pub mod compose_images;
pub mod container;
pub mod dockerapi;
pub mod dockercmd;
pub mod logs;

pub struct DockerRegistry {
    pub host: String,
}

#[cfg(test)]
mod tests {
    use std::boxed;

    use tokio_stream::StreamExt;

    async fn run(
        host: &str,
        user: Option<String>,
        passwd: Option<String>,
        image: &str,
    ) -> Result<(), boxed::Box<dyn std::error::Error>> {
        let client = docker_registry::v2::Client::configure()
            .registry(host)
            .insecure_registry(false)
            .username(user)
            .password(passwd)
            .build()?;

        let login_scope = format!("repository:{}:pull", image);
        let dclient = client.authenticate(&[&login_scope]).await?;

        // Get all tags (passing None instead of Some(7) for limit)
        dclient
            .get_tags(image, None)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(Result::unwrap)
            .for_each(|tag| {
                println!("{}", tag); // Print just the tag name without debug formatting
            });

        Ok(())
    }

    #[tokio::test]
    async fn test_tags() {
        let registry = "registry-1.docker.io";
        let image = "lagrangelabs/lagrange-node";

        println!("[{}] requesting tags for image {}", registry, image);

        // Optional authentication if needed
        let user = std::env::var("DKREG_USER").ok();
        let password = std::env::var("DKREG_PASSWD").ok();

        let res = run(registry, user, password, image).await;
        if let Err(e) = res {
            println!("[{}] {}", registry, e);
            std::process::exit(1);
        };
    }
}
