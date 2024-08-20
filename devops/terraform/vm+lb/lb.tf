resource "google_compute_health_check" "backend" {
  name               = "backend-basic-check"
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

resource "google_compute_backend_service" "backend" {
  name                            = "ivynet-backend-service"
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

# This seems to be a name for loadbalacner
resource "google_compute_url_map" "backend" {
  name            = "web-map-http"
  default_service = google_compute_backend_service.backend.id
}

resource "google_compute_target_https_proxy" "backend" {
  name             = "web-map-https"
  ssl_certificates = [google_compute_managed_ssl_certificate.api.id]
  url_map          = google_compute_url_map.backend.id
}

resource "google_compute_global_forwarding_rule" "backend" {
  name                  = "https-content-rule"
  ip_protocol           = "TCP"
  ip_address            = google_compute_global_address.loadbalancer.id
  load_balancing_scheme = "EXTERNAL_MANAGED"
  port_range            = "443"
  target                = google_compute_target_https_proxy.backend.id
}
