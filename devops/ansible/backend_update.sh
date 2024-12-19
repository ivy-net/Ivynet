#!/bin/sh
#
# The script to run ansible to update current backend
# Requires toml cargo package

remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"

echo "Find the backend version in Cargo.toml"
version=$(toml get ../../backend/Cargo.toml package.version |tr -d [\"])
echo " Version is: ${version}"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

cat << EOF
Run the playbook


 ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  -e "ivynet_backend_release=${version}" \\
  --vault-password-file ~/.vault.txt \\
  backend.yml
EOF

ansible-playbook -i gcp.yml -u ${remote_user} -e "ivynet_backend_release=${version}" --vault-password-file ~/.vault.txt backend.yml
