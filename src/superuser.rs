use anyhow::Result;
use newsletter_api::configuration::get_configuration;
use newsletter_api::models::{NewUser, NewUserData, UserProfile};
use newsletter_api::startup::get_connection_pool;
use secrecy::SecretString;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = get_connection_pool(&configuration.database);

    print!("Enter username: ");
    io::stdout().flush().unwrap();

    let mut username = String::new();
    io::stdin()
        .read_line(&mut username)
        .expect("Failed to read line");
    username = username.trim().to_string();

    print!("Enter email: ");
    io::stdout().flush().unwrap();

    let mut email = String::new();
    io::stdin()
        .read_line(&mut email)
        .expect("Failed to read line");
    email = email.trim().to_string();

    let mut password =
        rpassword::prompt_password("Enter password: ").expect("Failed to read line.");
    password = password.trim().to_string();

    let mut password_check =
        rpassword::prompt_password("Enter password again: ").expect("Failed to read line.");
    password_check = password_check.trim().to_string();

    if password_check == password {
        let new_user = NewUserData {
            email,
            username,
            password: SecretString::new(password_check.into()),
        };
        let mut transaction = connection_pool
            .begin()
            .await
            .expect("Failed to begin database transaction.");
        let user = NewUser::try_from(new_user)
            .expect("Failed to initialize user.")
            .make_superuser()
            .store(&mut transaction)
            .await
            .expect("Failed to store user.");
        UserProfile::initialize(&user.user_id)
            .insert(&mut transaction)
            .await
            .expect("Failed to create user profile");
        transaction
            .commit()
            .await
            .expect("Failed to commit transaction.");

        println!("Successfully stored user: {:#?}", user);
    }

    Ok(())
}
