#!/bin/bash

# RPM构建脚本 for runtime_measurer
# 使用方法: ./build_rpm.sh [version]

set -e

# 默认版本号
VERSION=${1:-"0.1.0"}
PACKAGE_NAME="runtime_measurer"
BUILD_DIR="$HOME/rpmbuild"
SPEC_FILE="${PACKAGE_NAME}.spec"
COPY_SOURCES=false

# 保存当前工作目录
ORIGINAL_DIR="$(pwd)"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查必要的工具
check_dependencies() {
    log_info "检查构建依赖..."
    
    local missing_deps=()
    
    # 检查rpm构建工具
    if ! command -v rpmbuild &> /dev/null; then
        missing_deps+=("rpm-build")
    fi
    
    if ! command -v rpmdev-setuptree &> /dev/null; then
        missing_deps+=("rpmdevtools")
    fi
    
    # 检查Rust工具链
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("rust cargo")
    fi
    
    # 检查protobuf编译器
    if ! command -v protoc &> /dev/null; then
        missing_deps+=("protobuf-compiler")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "缺少以下依赖: ${missing_deps[*]}"
        log_info "请运行以下命令安装依赖:"
        echo "sudo dnf install -y rpm-build rpmdevtools rust cargo protobuf-compiler protobuf-devel gcc"
        exit 1
    fi
    
    log_info "所有依赖检查通过"
}

# 设置RPM构建环境
setup_build_env() {
    log_info "设置RPM构建环境..."
    
    # 创建RPM构建目录结构
    rpmdev-setuptree
    
    # 确保目录存在
    mkdir -p "${BUILD_DIR}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
    
    log_info "RPM构建环境设置完成"
}

# 准备源码包
prepare_sources() {
    log_info "准备源码包..."
    
    local source_dir="${PACKAGE_NAME}-${VERSION}"
    local tarball="${PACKAGE_NAME}-${VERSION}.tar.gz"
    local temp_dir="/tmp/${source_dir}"
    
    # 清理之前的构建
    rm -rf "${temp_dir}" "${BUILD_DIR}/SOURCES/${tarball}"
    
    # 创建源码目录
    mkdir -p "${temp_dir}"
    
    # 复制源码文件（排除target目录和.git）
    log_info "复制源码文件到临时目录..."
    rsync -av --exclude='target' --exclude='.git' --exclude='*.rpm' \
          --exclude='rpmbuild' "${ORIGINAL_DIR}/" "${temp_dir}/"
    
    # 创建tar包（在临时目录的父目录中执行）
    log_info "创建源码包..."
    (
        cd /tmp
        tar czf "${BUILD_DIR}/SOURCES/${tarball}" "${source_dir}"
    )
    
    # 清理临时目录
    rm -rf "${temp_dir}"
    
    log_info "源码包准备完成: ${BUILD_DIR}/SOURCES/${tarball}"
}

# 复制spec文件
copy_spec() {
    log_info "复制spec文件..."
    
    # 确保在原始目录中查找spec文件
    local spec_path="${ORIGINAL_DIR}/${SPEC_FILE}"
    
    if [ ! -f "${spec_path}" ]; then
        log_error "找不到spec文件: ${spec_path}"
        exit 1
    fi
    
    cp "${spec_path}" "${BUILD_DIR}/SPECS/"
    log_info "spec文件复制完成"
}

# 构建RPM包
build_rpm() {
    log_info "开始构建RPM包..."
    
    # 在构建目录中执行构建
    (
        cd "${BUILD_DIR}"
        
        # 构建源码RPM
        log_info "构建源码RPM..."
        rpmbuild -bs "SPECS/${SPEC_FILE}"
        
        # 构建二进制RPM
        log_info "构建二进制RPM..."
        rpmbuild -bb "SPECS/${SPEC_FILE}"
    )
    
    log_info "RPM构建完成!"
}

# 显示构建结果
show_results() {
    log_info "构建结果:"
    
    echo "源码RPM:"
    find "${BUILD_DIR}/SRPMS" -name "*.rpm" -exec ls -lh {} \; 2>/dev/null || echo "未找到源码RPM"
    
    echo ""
    echo "二进制RPM:"
    find "${BUILD_DIR}/RPMS" -name "*.rpm" -exec ls -lh {} \; 2>/dev/null || echo "未找到二进制RPM"
    
    echo ""
    log_info "RPM包位置: ${BUILD_DIR}/RPMS/"
}

# 清理函数
cleanup() {
    if [ "$1" = "all" ]; then
        log_info "清理所有构建文件..."
        rm -rf "${BUILD_DIR}"
        log_info "清理完成"
    fi
}

# 错误处理函数
cleanup_on_error() {
    log_error "构建过程中发生错误，正在清理临时文件..."
    # 清理可能残留的临时目录
    rm -rf "/tmp/${PACKAGE_NAME}-${VERSION}"
    # 返回原始目录
    cd "${ORIGINAL_DIR}"
    exit 1
}

# 设置错误处理
trap cleanup_on_error ERR

# 复制文件到当前目录
copy_to_current_dir() {
    log_info "复制文件到当前目录..."
    
    local tarball="${PACKAGE_NAME}-${VERSION}.tar.gz"
    
    # 复制源码包
    if [ -f "${BUILD_DIR}/SOURCES/${tarball}" ]; then
        cp "${BUILD_DIR}/SOURCES/${tarball}" "${ORIGINAL_DIR}/"
        log_info "已复制源码包到: ${ORIGINAL_DIR}/${tarball}"
    else
        log_error "找不到源码包: ${BUILD_DIR}/SOURCES/${tarball}"
    fi
    
    # 复制spec文件
    if [ -f "${BUILD_DIR}/SPECS/${SPEC_FILE}" ]; then
        cp "${BUILD_DIR}/SPECS/${SPEC_FILE}" "${ORIGINAL_DIR}/"
        log_info "已复制spec文件到: ${ORIGINAL_DIR}/${SPEC_FILE}"
    else
        log_error "找不到spec文件: ${BUILD_DIR}/SPECS/${SPEC_FILE}"
    fi
}

# 显示帮助信息
show_help() {
    cat << EOF
RPM构建脚本 for ${PACKAGE_NAME}

使用方法:
    $0 [选项] [版本号]

选项:
    -h, --help      显示此帮助信息
    -c, --clean     清理构建目录
    --clean-all     清理所有构建文件
    --copy-sources  将源码包和spec文件复制到当前目录

示例:
    $0                  # 使用默认版本 ${VERSION}
    $0 0.2.0           # 使用指定版本
    $0 --clean         # 清理构建目录
    $0 --clean-all     # 清理所有构建文件
    $0 --copy-sources  # 复制源码包和spec文件到当前目录

当前工作目录: ${ORIGINAL_DIR}
构建目录: ${BUILD_DIR}

EOF
}

# 主函数
main() {
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -c|--clean)
            cleanup
            exit 0
            ;;
        --clean-all)
            cleanup all
            exit 0
            ;;
        --copy-sources)
            COPY_SOURCES=true
            shift
            ;;
        -*)
            log_error "未知选项: $1"
            show_help
            exit 1
            ;;
    esac
    
    log_info "开始构建 ${PACKAGE_NAME} RPM包 (版本: ${VERSION})"
    log_info "当前工作目录: ${ORIGINAL_DIR}"
    log_info "构建目录: ${BUILD_DIR}"
    
    check_dependencies
    setup_build_env
    prepare_sources
    copy_spec
    
    if [ "$COPY_SOURCES" = true ]; then
        copy_to_current_dir
        exit 0
    fi
    
    build_rpm
    show_results
    
    # 确保返回原始目录
    cd "${ORIGINAL_DIR}"
    
    log_info "构建完成! 🎉"
}

# 脚本入口
main "$@" 