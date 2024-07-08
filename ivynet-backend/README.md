# Testing

In order to receive emails from the platform you need to copy `.env.template` file to `.env` and provide proper Sendgrid keys and template ids.

Run the command from main repository folder
```
docker-compose  --env-file .env up -d
```

Dockers will expose both GRPC and HTTP ports.

To check available endpoints access `http://localhost:8080/swagger-ui` in your browser to check available endpoints documentation.
