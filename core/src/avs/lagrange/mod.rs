pub mod config;
pub mod setup;

//
// impl Lagrange {
//     async fn register(
//         &self,
//         _provider: Arc<IvyProvider>,
//         _eigen_path: PathBuf,
//         private_keypath: PathBuf,
//         keyfile_password: &str,
//     ) -> Result<(), IvyError> {
//         // Copy keyfile to current dir
//         let dest_dir = self.run_path().join("config");
//         if !dest_dir.exists() {
//             fs::create_dir_all(dest_dir.clone())?;
//         }
//         let dest_file = dest_dir.join("priv_key.json");
//
//         debug!("{}", dest_file.display());
//         fs::copy(private_keypath, &dest_file)?;
//         // Change dir to run docker file
//         std::env::set_current_dir(self.run_path())?;
//         // Set local env variable to pass password to docker
//         std::env::set_var("AVS__ETH_PWD", keyfile_password);
//         let _ = Command::new("docker")
//             .arg("compose")
//             .arg("run")
//             .args(["--rm", "worker", "avs", "register"])
//             .status()?;
//         fs::remove_file(dest_file)?;
//         Ok(())
//     }
// }
