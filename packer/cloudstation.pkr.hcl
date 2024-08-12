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
  zone                = "us-central1-a"
  image_family        = "ivynet-cloudstation"
  image_name          = "ivynet-cloudstation-${var.version}"
  disk_size = "40"
  ssh_username        = "packer"
  metadata = {
    "enable-oslogin" : "FALSE"
  }
}

build {
  sources = ["sources.googlecompute.ivynet-cloudstation"]

  provisioner "ansible" {
    playbook_file = "ansible/ivynet_client.yml"
    extra_arguments = ["--ask-vault-pass"]
  }
}
