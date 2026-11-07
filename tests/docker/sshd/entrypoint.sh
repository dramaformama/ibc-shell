#!/bin/bash
# MODE: pass | key | kbd | bastion | internal | evil
set -euo pipefail
MODE="${MODE:-pass}"
CFG=/etc/ssh/sshd_config.d/test.conf
mkdir -p /etc/ssh/sshd_config.d
grep -q '^Include' /etc/ssh/sshd_config || echo 'Include /etc/ssh/sshd_config.d/*.conf' >> /etc/ssh/sshd_config
# authorized_keys from mounted fixtures (public halves only)
: > /home/tester/.ssh/authorized_keys
for p in /fixtures/keys/*.pub; do [ -e "$p" ] && cat "$p" >> /home/tester/.ssh/authorized_keys; done
chown tester:tester /home/tester/.ssh/authorized_keys; chmod 600 /home/tester/.ssh/authorized_keys
{
  echo "LogLevel VERBOSE"
  echo "PermitRootLogin no"
  [ "$MODE" = bastion ] && echo "AllowTcpForwarding yes" || echo "AllowTcpForwarding no"
  case "$MODE" in
    pass|evil) echo "PasswordAuthentication yes"; echo "PubkeyAuthentication no"; echo "KbdInteractiveAuthentication no"; echo "UsePAM no" ;;
    key|internal) echo "PasswordAuthentication no"; echo "PubkeyAuthentication yes"; echo "KbdInteractiveAuthentication no"; echo "UsePAM no" ;;
    kbd) echo "PasswordAuthentication no"; echo "PubkeyAuthentication no"; echo "KbdInteractiveAuthentication yes"; echo "UsePAM yes"; echo "AuthenticationMethods keyboard-interactive" ;;
    bastion) echo "PasswordAuthentication no"; echo "PubkeyAuthentication yes"; echo "KbdInteractiveAuthentication no"; echo "UsePAM no" ;;
  esac
} > "$CFG"
passwd -u tester >/dev/null 2>&1 || true
if [ "$MODE" = kbd ]; then
  printf 'auth required pam_unix.so\naccount required pam_unix.so\nsession required pam_permit.so\n' > /etc/pam.d/sshd
fi

if [ "$MODE" = evil ]; then
  D=/home/tester/evil; mkdir -p "$D"; cd "$D"
  : > '..%2f..%2fx'; : > '..\..\x'; : > 'CON'; : > 'NUL.txt'; : > 'a\u0000b'; : > '-rf'; : > ' leading'; : > 'trailing.'
  : > "$(printf 'a%.0s' $(seq 255))"                                   # max-length name (FS limit 255 bytes)
  : > "$(printf 'cafe\xcc\x81-nfd')"; : > "$(printf 'caf\xc3\xa9-nfc')"  # NFD vs NFC
  : > "$(printf 'bad\xff\xfeutf8')"                                     # invalid UTF-8
  : > "$(printf 'new\nline')"; : > "$(printf 'tab\there')"
  ln -s loop_b loop_a; ln -s loop_a loop_b; ln -s ../../../etc/passwd escape_link; ln -s /etc/shadow abs_link
  mkdir -p big && ( cd big && seq 1 10000 | sed 's/^/f/' | xargs touch )
  chown -R -h tester:tester "$D"
fi
SSHD=/usr/sbin/sshd; [ "$MODE" = kbd ] && SSHD=/usr/sbin/sshd.pam
exec "$SSHD" -D -e
