# Testing

* Ensure that the sqlx-cli cargo package is installed, and the sqlx command is in the PATH.

* To run stack locally setup database with `docker-compose`:
```
docker-compose up -d
```

* When database is up the local `DATABASE_URL` environment variable has to be set to `postgresql://ivy:secret_ivy@localhost:5432/ivynet` and exported:
```
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
```
* Initial database migration has to be performed with command:
```
sqlx migrate run
```
* Once done, start the service. To do it with the test account use the test-account attribute:
```
cargo run -- --test-account=test@ivynet.dev:secret
```
* Checking the grpc enpoints can be a good initial test. The `grpcui` command (which has to be installed) can be used:
```
grpcui -plaintext 127.0.0.1:50050
```
