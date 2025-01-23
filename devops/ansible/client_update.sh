#!/bin/sh
#

echo "Find the client version in Cargo.toml"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"])
echo " Version/Release is: ${version}"
echo "Check if remote user is defined"
remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"


ansible-playbook \
  -i gcp.yml \
  -u ${remote_user} \
  --vault-password-file ~/.vault.txt \
  ivynet_client.yml
