resource "google_compute_network" "backend" {
  name                    = "backend"
  auto_create_subnetworks = false
  mtu                     = 1460
  routing_mode            = "REGIONAL"
}

resource "google_compute_subnetwork" "backend" {
  name                       = "backend"
  ip_cidr_range              = "10.0.1.0/24"
  network                    = google_compute_network.backend.id
  private_ipv6_google_access = "DISABLE_GOOGLE_ACCESS"
  purpose                    = "PRIVATE"
  stack_type                 = "IPV4_ONLY"
}

resource "google_compute_subnetwork" "proxy" {
  name          = "proxy"
  ip_cidr_range = "10.0.2.0/23"
  network       = google_compute_network.backend.id
  purpose       = "REGIONAL_MANAGED_PROXY"
  role          = "ACTIVE"
}

resource "google_compute_network_endpoint_group" "backend" {
  name         = "backend"
  network      = google_compute_network.backend.id
  subnetwork   = google_compute_subnetwork.backend.id
  default_port = "8080"
  zone         = "${var.region}-${var.zone}"
}

resource "google_compute_global_address" "loadbalancer" {
  name = "backend-loadbalancer"
}

resource "google_compute_managed_ssl_certificate" "api" {
  name = "backend"
  managed {
    domains = ["api1.${var.dns_zone}"]
  }
}

resource "google_dns_managed_zone" "test" {
  name = "test"
  dns_name = var.dns_zone
}

resource "google_dns_record_set" "backend" {
  name         = "api1.${var.dns_zone}"
  type         = "A"
  ttl          = 300
  managed_zone = google_dns_managed_zone.test.name
  rrdatas      = [google_compute_global_forwarding_rule.backend.ip_address]
}
