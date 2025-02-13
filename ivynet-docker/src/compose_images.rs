use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// Vector of ComposeImages. This is the main struct that will be used to store the parsed output
/// of `docker service ls`
#[derive(Debug, Clone)]
pub struct ComposeImages(pub Vec<ComposeImage>);

impl Deref for ComposeImages {
    type Target = Vec<ComposeImage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ComposeImages {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Singluar image output of `docker service ls`
#[derive(Debug, Clone)]
pub struct ComposeImage {
    pub container: String,
    pub repository: String,
    pub tag: String,
    pub image_id: String,
    pub size: String,
}

impl FromStr for ComposeImages {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let images = {
            let mut lines = s.lines();

            // Parse header to get column positions
            let header = lines.next().expect("Header line missing");
            let columns = ["CONTAINER", "REPOSITORY", "TAG", "IMAGE ID", "SIZE"];

            let positions: Vec<Option<usize>> =
                columns.iter().map(|&col| header.find(col)).collect();

            if positions.iter().any(|&pos| pos.is_none()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing column in header",
                ));
            }

            let positions: Vec<usize> = positions.into_iter().map(|pos| pos.unwrap()).collect();

            let images = lines
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
                .collect();
            ComposeImages(images)
        };
        Ok(images)
    }
}
