variable "project" {
  default     = "ivynet-tests"
  description = "Name of the GCP project"
  type        = string
}

variable "region" {
  default     = "us-central1"
  description = "Name of the region"
  type        = string
}

variable "zone" {
  default     = "c"
  description = "Letter for the zone (by default based on the region)"
  type        = string
}

variable "dns_zone" {
  default     = "test.ivynet.dev."
  description = "Zone for DNS and SSL"
  type        = string
}
