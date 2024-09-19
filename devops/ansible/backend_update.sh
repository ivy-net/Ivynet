#!/bin/sh
#
# The script to run ansible to update current backend
# Requires toml cargo package

remote_user=wawrzek_ivynet_dev

echo "Find the backend version in Cargo.toml"
version=$(toml get ../../backend/Cargo.toml package.version |tr -d [\"])
echo "Version is: ${version}"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

echo "Run the playbook"
echo "ansible-playbook -i gcp.yml --extra-vars ivynet_backend_release=${version} --vault-password-file ~/.vault.txt" backend.yml
ansible-playbook -i gcp.yml --extra-vars "ivynet_backend_release=${version} ansible_user=${remote_user}" --vault-password-file ~/.vault.txt backend.yml
