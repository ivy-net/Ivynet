/// Parsed output of `docker service ls`
#[derive(Debug)]
pub struct ComposeImage {
    pub container: String,
    pub repository: String,
    pub tag: String,
    pub image_id: String,
    pub size: String,
}

pub fn parse_docker_compose_images(output: &str) -> Vec<ComposeImage> {
    let mut lines = output.lines();

    // Parse header to get column positions
    let header = lines.next().expect("Header line missing");
    let columns = ["CONTAINER", "REPOSITORY", "TAG", "IMAGE ID", "SIZE"];
    let positions: Vec<_> = columns
        .iter()
        .map(|&col| header.find(col).expect(&format!("Column '{}' not found", col)))
        .collect();

    lines
        .filter_map(|line| {
            if line.trim().is_empty() {
                return None;
            }

            let mut image = ComposeImage {
                container: String::new(),
                repository: String::new(),
                tag: String::new(),
                image_id: String::new(),
                size: String::new(),
            };

            for i in 0..positions.len() {
                let start = positions[i];
                let end = positions.get(i + 1).copied().unwrap_or(line.len());
                let value = line[start..end].trim();

                match i {
                    0 => image.container = value.to_string(),
                    1 => image.repository = value.to_string(),
                    2 => image.tag = value.to_string(),
                    3 => image.image_id = value.to_string(),
                    4 => image.size = value.to_string(),
                    _ => unreachable!(),
                }
            }

            Some(image)
        })
        .collect()
}
