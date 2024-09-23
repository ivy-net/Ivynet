packer {
  required_plugins {
    googlecompute = {
      source  = "github.com/hashicorp/googlecompute"
      version = "~> 1"
    }
    ansible = {
      source  = "github.com/hashicorp/ansible"
      version = "~> 1"
    }
    sshkey = {
      source  = "github.com/ivoronin/sshkey"
      version = "~> 1"
    }
  }
}

variable "version" {
  type        = string
  default     = "dev"
  description = "Image version"
}

data "sshkey" "install" {
}

source "googlecompute" "ivynet-cloudstation" {
  source_image_family       = "ubuntu-2404-lts-amd64"
  project_id                = "ivynet-tests"
  zone                      = "us-central1-b"
  image_family              = "ivynet-cloudstation"
  image_name                = "ivynet-cloudstation-${var.version}"
  instance_name             = "packer-cloudstation-${var.version}"
  disk_size                 = "200"
  ssh_username              = "packer"
  ssh_private_key_file      = data.sshkey.install.private_key_path
  ssh_clear_authorized_keys = true
  labels = {
    "creator" : "packer",
    "area" : "client",
    "project" : "github_client"
  }
  metadata = {
    "enable-oslogin" : "FALSE"
  }
}

build {
  sources = ["sources.googlecompute.ivynet-cloudstation"]

  provisioner "ansible" {
    playbook_file = "../ansible/cloudstation-packer.yml"
    extra_arguments = [
      "--inventory",
      "../ansible/packer_gcp.yml",
      "-–private-key",
      data.sshkey.install.private_key_path,
      "--vault-password-file",
      "~/.vault.txt",
    ]
  }
}
