#!/bin/bash
# 发布脚本 - 自动更新版本号并发布
# 用法: ./scripts/release.sh [patch|minor|major]

set -e

VERSION_TYPE=${1:-patch}

# 检查是否在 main/master 分支
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" != "main" && "$BRANCH" != "master" ]]; then
    echo "Error: Must be on main or master branch"
    exit 1
fi

# 检查是否有未提交的更改
if [[ -n $(git status -s) ]]; then
    echo "Error: Working directory is not clean"
    exit 1
fi

# 获取当前版本
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "Current version: $CURRENT_VERSION"

# 解析版本号
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# 计算新版本
case $VERSION_TYPE in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
    *)
        echo "Error: Invalid version type. Use patch, minor, or major"
        exit 1
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
echo "New version: $NEW_VERSION"

# 更新 Cargo.toml
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml

# 运行测试
echo "Running tests..."
cargo test --all-features

# 提交更改
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"

# 创建标签
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo ""
echo "Version bumped to $NEW_VERSION"
echo ""
echo "To publish, run:"
echo "  git push origin $BRANCH --tags"
echo ""
echo "This will trigger the GitHub Actions workflow to publish to crates.io"
