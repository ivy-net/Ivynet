resource "google_compute_instance" "backend" {
  name = "backend"
  boot_disk {
    initialize_params {
      image = "ivynet-backend"
    }
  }
  machine_type = "n2-standard-2"
  network_interface {
    network    = google_compute_network.backend.id
    subnetwork = google_compute_subnetwork.backend.id
    access_config {}
  }
  zone = "${var.region}-${var.zone}"
  tags = ["ivynet-backend", "ssh"]
}
