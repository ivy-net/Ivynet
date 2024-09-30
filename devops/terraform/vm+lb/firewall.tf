resource "google_compute_firewall" "ssh" {
  name = "allow-ssh"
  allow {
    ports    = ["22"]
    protocol = "tcp"
  }
  direction     = "INGRESS"
  network       = google_compute_network.backend.id
  priority      = 1000
  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["ssh"]
}

resource "google_compute_firewall" "healh_check" {
  name = "allow-health-check"
  allow {
    protocol = "tcp"
  }
  direction     = "INGRESS"
  network       = google_compute_network.backend.id
  priority      = 1000
  source_ranges = ["130.211.0.0/22", "35.191.0.0/16"]
  target_tags   = ["ivynet-backend"]
}

resource "google_compute_firewall" "allow_proxy" {
  name = "allow-proxies"
  allow {
    ports    = ["443"]
    protocol = "tcp"
  }
  allow {
    ports    = ["80"]
    protocol = "tcp"
  }
  allow {
    ports    = ["8080"]
    protocol = "tcp"
  }
  allow {
    ports    = ["50050"]
    protocol = "tcp"
  }
  direction     = "INGRESS"
  network       = google_compute_network.backend.id
  priority      = 1000
  source_ranges = ["10.0.2.0/23"]
  target_tags   = ["ivynet-backend"]
}
