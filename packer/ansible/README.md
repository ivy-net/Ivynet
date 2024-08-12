# Roles

* ivynet-client -- base for cloudstation - clients binaries, but also rust
* ivynet-backend -- backend program + third party tools (memcached, postgresql)

## Testing

Default molecule scenario cannot test everything.
For client, it cannot download point files.
I tried GCE driver, but I cannot copy ssh over to run ansible.
In case of backend, postgres cannot be properly started (systemd).

### Client test
```
 molecule converge -- --tags github
```

## TODO

* Check if possible to use Ansible with GCE and OS-Login (Packer manage to do this)
* Start to export binaries outside of GitHub (it's going to make roles much easier)
