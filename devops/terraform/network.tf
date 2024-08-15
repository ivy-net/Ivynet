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
