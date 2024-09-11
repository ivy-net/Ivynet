#!/bin/sh
#
# The script to run ansible to update current backend

echo "Find the backend version in Cargo.toml"
version=$(toml get ../../backend/Cargo.toml package.version |tr -d [\"\])

echo "Activate Ansible"
source $HOME/bin/ansible/bin/activate

echo "Run the playbook"
echo "ansible-playbook -i inventory backend.yml --extra-vars ivynet_backend_release=${version} --vault-password-file ~/.vault.txt"
ansible-playbook -i inventory backend.yml --extra-vars ivynet_backend_release=${version} --vault-password-file ~/.vault.txt
