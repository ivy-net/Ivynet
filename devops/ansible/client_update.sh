#!/bin/sh
#

remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"

echo "Find the backend version in Cargo.toml"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"])
echo "Version/Release is: ${version}"

#  -e "ivynet_client_release=${version}" \
ansible-playbook \
  -i gcp.yml \
  -u ${remote_user} \
  --vault-password-file ~/.vault.txt \
  --tags github \
  ivynet_client.yml
