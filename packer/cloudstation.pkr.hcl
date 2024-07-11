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
  type        = string
  default     = "dev"
  description = "Image version"
}

source "googlecompute" "ivynet-cloudstation" {
  project_id          = "ivynet-tests"
  source_image_family = "ubuntu-2404-lts-amd64"
  image_name          = "ivynet-cloudstation-${var.version}"
  ssh_username        = "packer"
  zone                = "us-central1-a"
  metadata = {
    "enable-oslogin" : "FALSE"
  }
}

build {
  sources = ["sources.googlecompute.ivynet-cloudstation"]

  provisioner "file" {
    source      = "docker.list"
    destination = "/tmp/docker.list"
  }

  provisioner "file" {
    source      = "motd.txt"
    destination = "/tmp/motd"
  }

  provisioner "file" {
    source      = "environment"
    destination = "/tmp/environment"
  }

  provisioner "shell" {
    inline = [
      "sudo cp /tmp/motd /etc/",
      "sudo cp /tmp/docker.list /etc/apt/sources.list.d/",
      "sudo cp /tmp/environment /etc/",
    ]
  }
  provisioner "file" {
    source      = var.ssh_key_file
    destination = "/home/packer/.ssh/id_ed25519"
  }

  provisioner "shell" {
    inline = [
      "curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg",
      "sudo apt update",
      "sudo apt install -y rustc cargo protobuf-compiler pkg-config libssl-dev",
      "sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin",
      "ssh-keyscan github.com >> ~/.ssh/known_hosts",
      "sudo mkdir -p ${var.install_path}/bin",
      "sudo chmod 777 ${var.install_path}",
      "cd ${var.install_path}",
      "git clone git@github.com:ivy-net/ivynet.git -v",
//      "git clone https://github.com/Layr-Labs/eigenda.git", NOT SURE WE ACTUALLY NEED IT
      "git clone https://github.com/Layr-Labs/eigenda-operator-setup.git",
      "cd eigenda-operator-setup",
// TO DO REPLACE WITH MY METHOD OF DOWNLOADING g1, g2
//      "./srs_setup.sh",
      "curl -sSfL https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh | sudo sh -s -- -b ${var.install_path}/bin",
    ]
  }

}
