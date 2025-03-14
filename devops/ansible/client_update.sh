#!/bin/sh
#
# The script to run ansible to update current client
# Requires toml cargo package

while getopts m flag
do
  case "${flag}" in
    m) MASTER=True;;
  esac
done


echo "Check if remote user is defined"
remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"

echo "Find the client version in Cargo.toml"
version=$(toml get ../../cli/Cargo.toml package.version |tr -d [\"])
echo " Version/Release is: ${version}"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

if [[ ${MASTER} == 'True' ]]
then
cat << EOF
Run the playbook


ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  -e "ivynet_client_is_release=false" \\
  --vault-password-file ~/.vault.txt \\
  ivynet-client.yml
EOF

  ansible-playbook \
    -i gcp.yml \
    -u ${remote_user} \
    -e "ivynet_client_is_release=false" \
    --vault-password-file ~/.vault.txt \
    ivynet-client.yml
else
cat << EOF
Run the playbook


ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  --vault-password-file ~/.vault.txt \\
  ivynet-client.yml
EOF

  ansible-playbook \
    -i gcp.yml \
    -u ${remote_user} \
    --vault-password-file ~/.vault.txt \
    ivynet-client.yml
fi
