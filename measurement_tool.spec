%define alinux_release 1
%global config_dir /etc/measurement_tool

Name:           measurement_tool
Version:        0.2.0
Release:        %{alinux_release}%{?dist}
Summary:        Runtime measurement tool for confidential computing environments
Group:          Applications/System
ExclusiveArch:  x86_64

License:        Apache-2.0
URL:            https://github.com/inclavare-containers/measurement_tools
Source0:        %{name}-%{version}.tar.gz
Source1:        vendor.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  protobuf-compiler
BuildRequires:  protobuf-devel

Requires:       attestation-agent

%global debug_package %{nil}
%global _build_id_links none

%global __requires_exclude_from ^%{_bindir}/.*$

%description
measurement tool is a flexible runtime measurement tool for confidential 
computing environments that measures various system resources and communicates 
with Attestation Agents via ttrpc protocol. It supports file measurements, 
process measurements, and container image measurements.

%prep
%autosetup -n measurement_tool-%{version}
tar xf %{SOURCE1} -C %{_builddir}/measurement_tool-%{version}

%build
export CARGO_HOME=%{_builddir}/.cargo
# NOTE:
# - `-C target-cpu=native` 会把构建机的 CPU 指令集特性固化进产物，换机器/换虚拟化环境可能触发 SIGILL。
# - 这里使用更通用的 x86-64 baseline，确保在更广泛的 x86_64 机器上可运行。
export RUSTFLAGS="-C opt-level=3 -C target-cpu=x86-64"
export CARGO_VENDOR_DIR=%{_builddir}/measurement_tool-%{version}/vendor

cargo build --release --locked --offline

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{config_dir}
mkdir -p %{buildroot}%{_prefix}/lib/systemd/system

install -m 755 target/release/measurement_tool %{buildroot}%{_bindir}/measurement_tool
install -m 644 config.example.toml %{buildroot}%{config_dir}/config.toml
install -m 644 measurement_tool.service %{buildroot}%{_prefix}/lib/systemd/system/measurement_tool.service

%files
%doc README.md
%{_bindir}/measurement_tool
%config(noreplace) %{config_dir}/config.toml
%{_prefix}/lib/systemd/system/measurement_tool.service

%post
systemctl daemon-reload

%preun
if [ $1 == 0 ]; then #uninstall
  systemctl unmask measurement_tool
  systemctl stop measurement_tool
  systemctl disable measurement_tool
fi

%postun
if [ $1 == 0 ]; then #uninstall
  systemctl daemon-reload
  systemctl reset-failed
fi

%changelog
* Thu Jul 17 2025 Weidong Sun <sunweidong@linux.alibaba.com> - 0.2.0-1
- Update eventlog format

* Tue Jun 24 2025 Weidong Sun <sunweidong@linux.alibaba.com> - 0.1.0-2
- Remove rust version restriction

* Fri May 30 2025 Weidong Sun <sunweidong@linux.alibaba.com> - 0.1.0-1
- Initial package release
- Runtime measurement tool for confidential computing
- Support for file measurements
- Integration with attestation-agent via ttrpc protocol 