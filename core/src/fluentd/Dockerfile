FROM fluent/fluentd:v1.17-1
# Switch to root to install plugins
USER root

# Install build tools and dependencies
RUN apk add --no-cache --update \
    build-base \
    ruby-dev \
    && gem install fluent-plugin-grpc \
    && gem sources --clear-all \
    && apk del build-base ruby-dev \
    && rm -rf /tmp/* /var/tmp/* /usr/lib/ruby/gems/*/cache/*.gem

RUN mkdir -p /fluentd/log && \
    chown fluent:fluent /fluentd/log

# Switch back to fluent user
USER fluent
