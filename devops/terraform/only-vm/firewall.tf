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

resource "google_compute_firewall" "api" {
  name = "allow-backend-api"
  allow {
    ports    = ["8080"]
    protocol = "tcp"
  }
  direction     = "INGRESS"
  network       = google_compute_network.backend.id
  priority      = 1000
  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["ivynet-backend"]
}

resource "google_compute_firewall" "http" {
  name = "allow-backend-http"
  allow {
    ports    = ["80"]
    protocol = "tcp"
  }
  direction     = "INGRESS"
  network       = google_compute_network.backend.id
  priority      = 1000
  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["ivynet-backend"]
}
