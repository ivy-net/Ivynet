resource "google_compute_health_check" "backend" {
  name               = "backend-http-check"
  check_interval_sec = 5
  healthy_threshold  = 2
  http_health_check {
    port               = 8080
    port_specification = "USE_FIXED_PORT"
    proxy_header       = "NONE"
    request_path       = "/health"
  }
  timeout_sec         = 5
  unhealthy_threshold = 2
}

resource "google_compute_backend_service" "http" {
  name                            = "ivynet-http-service"
  connection_draining_timeout_sec = 0
  health_checks                   = [google_compute_health_check.backend.id]
  load_balancing_scheme           = "EXTERNAL_MANAGED"
  port_name                       = "http"
  protocol                        = "HTTP"
  session_affinity                = "NONE"
  timeout_sec                     = 30
  backend {
    group           = google_compute_instance_group.backend.id
    balancing_mode  = "UTILIZATION"
    capacity_scaler = 1.0
  }
}

resource "google_compute_backend_service" "grpc" {
  name                            = "ivynet-grpc-service"
  connection_draining_timeout_sec = 0
  health_checks                   = [google_compute_health_check.backend.id]
  load_balancing_scheme           = "EXTERNAL_MANAGED"
  port_name                       = "grpc"
  protocol                        = "HTTP2"
  session_affinity                = "NONE"
  timeout_sec                     = 30
  backend {
    group           = google_compute_instance_group.backend.id
    balancing_mode  = "UTILIZATION"
    capacity_scaler = 1.0
  }
}

# This seems to be a name for loadbalacner
resource "google_compute_url_map" "http" {
  name            = "web-map-http"
  default_service = google_compute_backend_service.http.id
}

resource "google_compute_url_map" "grpc" {
  name            = "web-map-grpc"
  default_service = google_compute_backend_service.grpc.id
}

resource "google_compute_target_https_proxy" "http" {
  name             = "web-map-http"
  url_map          = google_compute_url_map.http.id
  ssl_certificates = [google_compute_managed_ssl_certificate.api.id]
}

resource "google_compute_target_https_proxy" "grpc" {
  name             = "web-map-grpc"
  url_map          = google_compute_url_map.grpc.id
  ssl_certificates = [google_compute_managed_ssl_certificate.api.id]
}

resource "google_compute_global_forwarding_rule" "http" {
  name                  = "https-content-rule"
  ip_protocol           = "TCP"
  ip_address            = google_compute_global_address.loadbalancer.id
  load_balancing_scheme = "EXTERNAL_MANAGED"
  port_range            = "443"
  target                = google_compute_target_https_proxy.http.id
}

resource "google_compute_global_forwarding_rule" "grpc" {
  name                  = "grpc-content-rule"
  ip_protocol           = "TCP"
  ip_address            = google_compute_global_address.loadbalancer.id
  load_balancing_scheme = "EXTERNAL_MANAGED"
  port_range            = "50050"
  target                = google_compute_target_https_proxy.grpc.id
}
