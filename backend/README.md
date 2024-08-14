# Full deployment testing

## Test with docker compose

To simplify tests the docker compose file with the backend as well all third party dependencies (e.g. memcached or postgres) is provided.
Run the command from current repository folder to use it;
```sh
docker-compose  --env-file .env up -d
```
Dockers will expose both GRPC and HTTP ports.

To check available endpoints access `http://localhost:8080/swagger-ui` in your browser to check available endpoints documentation.

## Testing email functionalities

Backend depends on sengrid to send emails.
In order to receive emails from the platform you need to copy `.env.template` file to `.env` and provide proper Sendgrid keys and template ids.


# Backend SQL changes

* Ensure that the _sqlx-cli_ cargo package is installed, and the sqlx command is in the PATH.

If any of the SQL commands change (in files in `src` folder) the sqlx cache has to be updated.
Otherwise tools like `cargo clippy` are going to be confused and fail.

* Run the `docker-compose` with the `backend-compose.yaml` configuration to start the postgres SQL:

```sh
docker-compose -f backend-compose.yaml up  -d

 ```

* When database is up, point the `DATABASE_URL` environment variable onto it:

```sh
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
```

* Prepare migrations with the command:

```sh
sqlx migrate prepare
```
