#!/bin/sh
#
# The script to run ansible to update current backend
# Requires toml cargo package

echo "Check if remote user is defined"
remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"

echo "Find the client version in Cargo.toml"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"])
echo " Version/Release is: ${version}"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

cat << EOF
Run the playbook


ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  --vault-password-file ~/.vault.txt \\
  ivynet_client.yml
EOF

ansible-playbook -i gcp.yml -u ${remote_user} --vault-password-file ~/.vault.txt ivynet_client.yml
