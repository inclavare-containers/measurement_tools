#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
mkdir -p /work /build/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
tar -xzf /output/source.tar.gz -C /work
cd /work
arch=$(rpm --eval '%{_arch}')
bash build_rpm.sh
cp -a /root/rpmbuild/RPMS /root/rpmbuild/SRPMS /build/

# Check package metadata and executable dependencies on the actual build platform.
for package in /build/RPMS/"$arch"/*.rpm; do
  test "$(rpm -qp --qf '%{ARCH}' "$package")" = "$arch"
done
# The AA runtime is packaged in a separate repository; install without resolving it.
rpm -i --nodeps --noscripts /build/RPMS/"$arch"/measurement_tool-*.rpm
cat > /tmp/measurement-smoke.toml <<'EOF'
one_shot = true
aa_channel = "http_api"
trustiflux_api_endpoint = "http://127.0.0.1:1"
EOF
# Disabled measurers exercise native startup/configuration without an AA server.
measurement_tool /tmp/measurement-smoke.toml
mkdir -p /output/RPMS /output/SRPMS
cp -a /build/RPMS/. /output/RPMS/
cp -a /build/SRPMS/. /output/SRPMS/
