%define alinux_release 1
%global config_dir /etc/measurement_tool

Name:           measurement_tool
Version:        0.1.0
Release:        %{alinux_release}%{?dist}
Summary:        Runtime measurement tool for confidential computing environments
Group:          Applications/System
BuildArch:      x86_64

License:        Apache-2.0
URL:            https://github.com/inclavare-containers/measurement_tools
Source0:        %{name}-%{version}.tar.gz
Source1:        vendor.tar.gz

BuildRequires:  rust = 1.75.0
BuildRequires:  cargo = 1.75.0
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
export RUSTFLAGS="-C opt-level=3 -C target-cpu=native"
export CARGO_VENDOR_DIR=%{_builddir}/measurement_tool-%{version}/vendor

cargo build --release --locked --offline

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_sysconfdir}/measurement_tool
mkdir -p %{buildroot}%{_unitdir}

install -m 755 target/release/measurement_tool %{buildroot}%{_bindir}/measurement_tool
install -m 644 config.example.toml %{buildroot}%{_sysconfdir}/measurement_tool/config.toml
install -m 644 measurement_tool.service %{buildroot}%{_unitdir}/measurement_tool.service

%files
%doc README.md
%{_bindir}/measurement_tool
%config(noreplace) %{_sysconfdir}/measurement_tool/config.toml
%{_unitdir}/measurement_tool.service

%post
%systemd_post measurement_tool.service

%preun
%systemd_preun measurement_tool.service

%postun
%systemd_postun_with_restart measurement_tool.service

%changelog
* Fri May 30 2025 Weidong Sun <sunweidong@linux.alibaba.com> - 0.1.0-1
- Initial package release
- Runtime measurement tool for confidential computing
- Support for file measurements
- Integration with attestation-agent via ttrpc protocol 