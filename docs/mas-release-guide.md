# Kraph 上架 Mac App Store 完整指南

> 适用版本：Tauri v2 + Vue 3  
> Bundle ID：`me.kraph.app`  
> 证书 Team ID：`4J7V324U38`

---

## 前置条件

- 已加入 [Apple Developer Program](https://developer.apple.com)（$99/年）
- 已安装 Rust 工具链和 pnpm

---

## 第一步：在 App Store Connect 创建 App 记录

1. 前往 [appstoreconnect.apple.com](https://appstoreconnect.apple.com)
2. 「我的 App」→「+」→「新建 App」
3. 填写信息：
   - 平台：macOS
   - 名称：Kraph
   - Bundle ID：`me.kraph.app`
   - SKU：自定义，如 `kraph-001`
4. 进入 App 详情页，完善截图、描述、关键词、隐私政策 URL 等

---

## 第二步：创建证书

### 2.1 生成 CSR 文件

1. 打开 **钥匙串访问（Keychain Access）**
2. 菜单栏：钥匙串访问 → 证书助理 → 从证书颁发机构请求证书
3. 填写 Apple ID 邮箱，选择「存储到磁盘」，保存 `.certSigningRequest` 文件

### 2.2 在 Developer Portal 创建证书

前往 [developer.apple.com/account/resources/certificates](https://developer.apple.com/account/resources/certificates)，分别创建：

| 证书类型 | 用途 |
|---------|------|
| Mac App Distribution | 签名 .app 文件 |
| Mac Installer Distribution | 签名 .pkg 安装包 |

每次创建时上传 CSR 文件，下载后**双击安装**到钥匙串。

### 2.3 安装 Apple 中间证书

如果 `security find-identity -v -p codesigning` 显示 0 个有效证书，需要安装中间证书：

```bash
curl -O https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer && open AppleWWDRCAG3.cer
```

### 2.4 验证证书安装

```bash
# 验证签名证书
security find-identity -v -p codesigning

# 查看所有证书（含 Installer 证书）
security find-identity -v
```

正确输出示例：
```
1) XXXXXXXX "Apple Distribution: zhoufeng dai (4J7V324U38)"
2) XXXXXXXX "3rd Party Mac Developer Installer: zhoufeng dai (4J7V324U38)"
```

---

## 第三步：创建 Provisioning Profile

1. 前往 [developer.apple.com/account/resources/profiles](https://developer.apple.com/account/resources/profiles)
2. 点击「+」→ 选择 **Mac App Store Connect**
3. 选择 Bundle ID：`me.kraph.app`
4. 选择证书：Apple Distribution
5. 命名为 `Kraph MAS Profile`，下载保存

---

## 第四步：配置 Tauri 项目

### 4.1 创建沙盒权限文件 `src-tauri/Entitlements.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.device.microphone</key>
    <true/>
</dict>
</plist>
```

> 根据实际权限需求增减，常用权限参考 [Apple 文档](https://developer.apple.com/documentation/bundleresources/entitlements)

### 4.2 更新 `src-tauri/tauri.conf.json`

在 `bundle.macOS` 节点添加签名配置：

```json
"macOS": {
  "infoPlist": "Info.plist",
  "minimumSystemVersion": "10.15",
  "entitlements": "./Entitlements.plist",
  "signingIdentity": "Apple Distribution: zhoufeng dai (4J7V324U38)",
  "providerShortName": "4J7V324U38"
}
```

---

## 第五步：构建应用

> 每次上传新包前，确保 `package.json` 中的 `version` 字段已更新。

### 5.1 安装 Rust 跨架构 Target（首次需要）

```bash
rustup target add x86_64-apple-darwin
```

### 5.2 构建 Universal Binary

```bash
pnpm tauri build --target universal-apple-darwin
```

构建产物路径：
```
src-tauri/target/universal-apple-darwin/release/bundle/macos/Kraph.app
```

---

## 第六步：打包 .pkg 安装包

```bash
productbuild \
  --component ~/Documents/mine/memoryai/src-tauri/target/universal-apple-darwin/release/bundle/macos/Kraph.app \
  /Applications \
  --sign "3rd Party Mac Developer Installer: zhoufeng dai (4J7V324U38)" \
  ~/Desktop/Kraph.pkg
```

---

## 第七步：上传到 App Store Connect

使用 **Transporter** 应用（App Store 免费下载）：

1. 打开 Transporter，登录 Apple ID
2. 将 `Kraph.pkg` 拖入窗口
3. 点击「交付」，等待上传完成

---

## 第八步：提交审核

1. 回到 App Store Connect，进入 Kraph 的 App 详情页
2. 刷新后在「构建版本」中选择刚上传的版本
3. 确认所有信息填写完整（截图、描述、隐私政策等）
4. 点击「提交以供审查」
5. 等待 Apple 审核（通常 1-3 个工作日）

---

## 日常更新发布流程

每次发布新版本，按以下顺序操作：

```bash
# 1. 更新 package.json 中的 version

# 2. 构建
pnpm tauri build --target universal-apple-darwin

# 3. 打包
productbuild \
  --component ~/Documents/mine/memoryai/src-tauri/target/universal-apple-darwin/release/bundle/macos/Kraph.app \
  /Applications \
  --sign "3rd Party Mac Developer Installer: zhoufeng dai (4J7V324U38)" \
  ~/Desktop/Kraph.pkg

# 4. 用 Transporter 上传，然后在 App Store Connect 提交审核
```

---

## 常见问题

| 问题 | 原因 | 解决方法 |
|------|------|---------|
| `0 valid identities found` | 缺少 Apple 中间证书 | 安装 AppleWWDRCAG3.cer |
| `Target x86_64-apple-darwin is not installed` | 缺少 Rust target | `rustup target add x86_64-apple-darwin` |
| Transporter 上传失败 | 版本号重复 | 更新 `package.json` 中的 `version` |
| 沙盒权限被拒 | Entitlements 缺少声明 | 在 `Entitlements.plist` 添加对应权限 |
