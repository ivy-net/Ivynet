use std::path::PathBuf;

pub fn make_fluentd_conf(ivynet_home: PathBuf) {
    let contents = include_str!("conf/fluent.conf");

    let fluentd_conf = get_fluentd_path(ivynet_home).join("conf/fluent.conf");
    if !&fluentd_conf.exists() {
        std::fs::create_dir_all(fluentd_conf.parent().unwrap())
            .expect("Unable to create directory");
    }

    // dump to file
    std::fs::write(fluentd_conf, contents).expect("Unable to write file");
}

pub fn make_fluentd_compose(ivynet_home: PathBuf) {
    let contents = include_str!("docker-compose.yml");

    let fluentd_path = get_fluentd_path(ivynet_home);
    if !&fluentd_path.exists() {
        std::fs::create_dir_all(&fluentd_path).expect("Unable to create directory");
    }

    let fluentd_compose = fluentd_path.join("docker-compose.yml");

    // dump to file
    std::fs::write(fluentd_compose, contents).expect("Unable to write file");
}

pub fn make_fluentd_dockerfile(ivynet_home: PathBuf) {
    let contents = include_str!("Dockerfile");

    let fluentd_path = get_fluentd_path(ivynet_home);
    if !&fluentd_path.exists() {
        std::fs::create_dir_all(&fluentd_path).expect("Unable to create directory");
    }

    let fluentd_dockerfile = fluentd_path.join("Dockerfile");
    // dump to file
    std::fs::write(fluentd_dockerfile, contents).expect("Unable to write file");
}

fn get_fluentd_path(ivynet_home: PathBuf) -> PathBuf {
    ivynet_home.join("fluentd")
}

#[test]
fn test_integration_fluentd() {
    let tempfile = tempfile::tempdir().unwrap();
    let filepath = tempfile.path();
    make_fluentd_dockerfile(filepath.to_path_buf());
    make_fluentd_conf(filepath.to_path_buf());
    make_fluentd_compose(filepath.to_path_buf());
    let fluentd_path = get_fluentd_path(filepath.to_path_buf());
    // print contents of fluentd_path
    let fluentd_path_contents = std::fs::read_dir(&fluentd_path).unwrap();
    for entry in fluentd_path_contents {
        let entry = entry.unwrap();
        println!("{:?}", entry.path());
    }
    let up_cmd = std::process::Command::new("docker")
        .arg("compose")
        .arg("up")
        .current_dir(fluentd_path)
        .output()
        .expect("failed to execute process");
    assert!(up_cmd.status.success());
}
