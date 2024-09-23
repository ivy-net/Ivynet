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
  }
}

variable "version" {
  type        = string
  default     = "dev"
  description = "Image version"
}

source "googlecompute" "ivynet-backend" {
  source_image_family = "ubuntu-2404-lts-amd64"
  project_id          = "ivynet-tests"
  zone                = "us-central1-b"
  image_family        = "ivynet-backend"
  image_name          = "ivynet-backend-${var.version}"
  instance_name       = "packer-backend-${var.version}"
  disk_size           = "200"
  ssh_username        = "packer"
  labels = {
    "creator" : "packer",
    "area" : "backend",
    "project" : "github_backend"
  }
  metadata = {
    "enable-oslogin" : "FALSE"
  }
}

build {
  sources = ["sources.googlecompute.ivynet-backend"]

  provisioner "ansible" {
    playbook_file = "../ansible/backend-packer.yml"
    extra_arguments = [
      "--vault-password-file",
      "~/.vault.txt",
      "--extra-vars",
      "{'ivynet_backend': 'backend-${var.version}'}"
    ]
  }
}
