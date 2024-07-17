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

/*  provisioner "shell" {
    inline = [
      "systemctl status sshd",
      "cat /etc/ssh/sshd_config",
      "cat .ssh/id_rsa.pub > .ssh/authorized_keys",
      "ssh localhost -v",
      "ssh-keyscan localhost >> ~/.ssh/known_hosts",
    ]
  }
*/
  provisioner "ansible" {
    playbook_file = "ansible/ivynet_client.yml"
  }
}
