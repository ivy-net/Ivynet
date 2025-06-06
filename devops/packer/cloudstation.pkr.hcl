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

source "googlecompute" "ivynet-cloudstation" {
  source_image_family = "ubuntu-2404-lts-amd64"
  project_id          = "ivynet-tests"
  zone                = "us-central1-b"
  image_family        = "ivynet-cloudstation"
  image_name          = "ivynet-cloudstation-${var.version}"
  instance_name       = "packer-cloudstation-${var.version}"
  disk_size           = "200"
  ssh_username        = "packer"
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
    ansible_env_vars = [
      "ANSIBLE_PIPELINING=true",
      "ANSIBLE_VAULT_PASSWORD_FILE=~/.vault.txt"
    ]
  }
}
