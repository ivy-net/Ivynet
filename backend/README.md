# Full deployment testing

In order to receive emails from the platform you need to copy `.env.template` file to `.env` and provide proper Sendgrid keys and template ids.

Run the command from main repository folder
```
docker-compose  --env-file .env up -d
```

Dockers will expose both GRPC and HTTP ports.

To check available endpoints access `http://localhost:8080/swagger-ui` in your browser to check available endpoints documentation.

# Backend SQL changes

* Ensure that the sqlx-cli cargo package is installed, and the sqlx command is in the PATH.

If any of the SQL commands change (in files in `src` folder) the sqlx cache has to be updated.
Otherwise tools like `cargo clippy` are going to be confused and fail.

* Run `docker-compose` to start the postgres SQL:
 ```
docker-compose -f backend-compose.yaml up  -d
 ```
* When database is up, point the `DATABASE_URL` environment variable onto it:
```
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
```
* Prepare migrations with the command:
```
sqlx migrate prepare
```
