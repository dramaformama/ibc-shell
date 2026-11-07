# Test-only OpenSSH server. Pin by digest when refreshing: docker buildx imagetools inspect alpine:3.20
FROM alpine:3.20
RUN apk add --no-cache openssh-server openssh-server-pam linux-pam bash coreutils \
 && ssh-keygen -A \
 && adduser -D -s /bin/bash tester \
 && echo 'tester:test-pw-123' | chpasswd \
 && mkdir -p /home/tester/.ssh /run/sshd && chown -R tester:tester /home/tester/.ssh && chmod 700 /home/tester/.ssh
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
EXPOSE 22
ENTRYPOINT ["/entrypoint.sh"]
