use dialoguer::Password;

pub fn get_confirm_password() -> String {
    let mut pw: String =
        Password::new().with_prompt("Enter a password for keyfile encryption").interact().unwrap();
    let mut confirm_pw: String =
        Password::new().with_prompt("Confirm keyfile password").interact().unwrap();

    let mut pw_confirmed = pw == confirm_pw;
    while !pw_confirmed {
        println!("Password and confirmation do not match. Please retry.");
        pw = Password::new()
            .with_prompt("Enter a password for keyfile encryption")
            .interact()
            .unwrap();
        confirm_pw = Password::new().with_prompt("Confirm keyfile password").interact().unwrap();
        pw_confirmed = pw == confirm_pw;
    }
    pw
}
