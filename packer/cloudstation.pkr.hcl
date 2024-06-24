packer {
  required_plugins {
    googlecompute = {
      source  = "github.com/hashicorp/googlecompute"
      version = "~> 1"
    }
  }
}

variable "install_path" {
  type        = string
  default     = "/opt/eigen"
  description = "Path to clone eigen repositories"
}

variable "ssh_key_file" {
  type        = string
  default     = "packer.prv"
  description = "Local path to the file with ssh key (should be ed25519)"
}

variable "version" {
  type = string
  default = "dev"
  description = "Image version"
}

source "googlecompute" "ivynet-cloudstation" {
  project_id   = "ivynet-tests"
  source_image_family = "ubuntu-2404-lts-amd64"
  image_name = "ivynet-cloudstation-${var.version}"
  ssh_username = "packer"
  zone         = "us-central1-a"
  metadata = {
    "enable-oslogin" : "FALSE"
  }
}

build {
  sources = ["sources.googlecompute.ivynet-cloudstation"]

  provisioner "file" {
    source      = var.ssh_key_file
    destination = "/home/packer/.ssh/id_ed25519"
  }

  provisioner "shell" {
    inline = [
      "sudo apt install rustc cargo -y",
      "ssh-keyscan github.com >> ~/.ssh/known_hosts",
      "sudo mkdir ${var.install_path}",
      "sudo chmod 777 ${var.install_path}",
      "cd ${var.install_path}",
      "git clone git@github.com:ivy-net/ivynet.git -v",
      "git clone https://github.com/Layr-Labs/eigenda.git",
      "git clone https://github.com/Layr-Labs/eigenda-operator-setup.git",
      "cd eigenda-operator-setup",
      "./srs_setup.sh",
    ]
  }

  provisioner "file" {
    source      = "motd.txt"
    destination = "/tmp/motd"
  }
  provisioner "shell" {
    inline = [
      "sudo cp /tmp/motd /etc"
    ]
  }
}
