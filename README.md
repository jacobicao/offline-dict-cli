# offline-dict-cli

一个给学生用的、安安静静的离线查词小工具。

它只有一件事：在 Windows 终端里，快速查一个英文单词，或者用中文反查英文单词。  
不联网，不弹广告，不猜你的意思，也不绕弯子。

适合这些场景：

- 上自习时想快速查词，不想打开浏览器
- 做四级 / 六级 / 考研阅读时，想看一个词是不是高频
- 写代码、写作业、刷题时，想用命令行秒查

## 它长什么样

查英文：

```text
dict abandon

abandon
tags: CET4 CET6
1. 放任
2. 狂热
3. 遗弃
4. 放弃
5. 丢弃
...
```

中文反查：

```text
dict 放弃

放弃
1. drop
2. abandon
3. compromise
4. desert
5. discard

5 of 45 results, use --all to show more
```

看全部结果：

```text
dict --all 放弃
```

## 这个工具适合谁

- 喜欢简单工具的学生
- 经常背英语单词的人
- 平时就在终端里工作的同学
- 不想被“智能推荐”“模糊搜索”打扰的人

## 特点

- 离线可用：断网也能查
- 单文件分发：下载一个 `dict.exe` 就能用
- 启动快：不是数据库程序，也不需要初始化
- 结果直接：精确匹配，不猜你想查什么
- 英文带标签：方便判断是不是常见词、四六级词
- 中文反查可排序：更常见、更考试向的词会靠前

## 当前规则

当前内置词库是“单词版”，不包含短语。

- 英文查询：大小写不敏感，按英文单词精确匹配
- 中文查询：按简体中文释义精确匹配
- 默认最多显示 5 条中文反查结果
- `--all` 可以显示全部结果
- 没找到时会输出 `未找到精确匹配: <query>`

也就是说：

- `dict Apple` 可以查到 `apple`
- `dict apples` 不会自动变成 `apple`
- `dict abandon ship` 现在不会命中内置词库

## 下载

已经提供 Windows 下载包，直接去这里下载：

- [GitHub Releases](https://github.com/jacobicao/offline-dict-cli/releases)

现在推荐两种下载方式：

1. `dict-setup-x64.msi`
   
   适合大多数同学。双击安装，会把程序装到用户目录，并把命令加入当前用户的 `PATH`。

2. `dict-windows-x86_64.zip`
   
   适合想自己放置 `dict.exe` 的同学。

### 如果你下载的是 MSI

安装完成后，重新打开 PowerShell / Windows Terminal，然后直接：

```powershell
dict apple
```

### 如果你下载的是 ZIP

解压后把 `dict.exe` 放到你喜欢的位置。

如果你只是临时在当前目录测试，可以：

```powershell
.\dict.exe apple
```

## 命令

```text
dict <query>
dict --all <query>
dict --help
dict --version
```

## 现在有哪些标签

当前生成数据里主要会看到这些标签：

- `COMMON_3500`
- `CET4`
- `CET6`

代码层面已经预留了更多标签位，但如果词源里没有对应数据，就不会显示出来。README 这里按当前真实情况写，不做超额承诺。

## 为什么它故意做得很“笨”

因为查词工具越聪明，很多时候反而越打扰。

这个项目故意不做这些功能：

- 模糊搜索
- 联想词
- 词形还原
- 拼音
- 例句
- 发音
- 在线翻译
- 用户词库热加载

如果你需要一个“大而全”的词典，这个项目不适合你。  
如果你想要一个“输入就出结果”的小工具，它应该挺合适。

## 自己构建

如果你想自己生成内置词库并编译：

### 1. 准备词库源

本项目当前使用外部公开词库仓库作为构建输入：

```powershell
git clone https://github.com/KyleBing/english-vocabulary.git
```

### 2. 生成本地数据

```powershell
cargo run --bin generate_dataset -- D:\git\english-vocabulary
```

这一步会在本地生成：

```text
data/generated/dictionary.json
```

这个文件只作为本地构建输入，不需要提交到仓库。

### 3. 编译 release

```powershell
cargo build --release
```

产物在：

```text
target/release/dict.exe
```

### 4. 生成 MSI 安装包

如果你是在 Windows 上，并且已经装好 WiX Toolset：

```powershell
cargo install cargo-wix --locked
cargo wix
```

MSI 产物会在：

```text
target/wix/
```

## 开发说明

项目现在的结构很小：

```text
src/main.rs
src/lib.rs
src/importer.rs
src/bin/generate_dataset.rs
tests/
build.rs
```

大致分工：

- `src/main.rs`：CLI 入口
- `src/lib.rs`：查询引擎、格式化、数据加载
- `src/importer.rs`：把外部词库清洗成可内置的数据
- `src/bin/generate_dataset.rs`：本地生成 `dictionary.json`
- `build.rs`：构建时把本地生成的数据嵌进最终二进制
- `wix/main.wxs`：Windows MSI 安装器配置

## 测试

```powershell
cargo test -j 1
```

## 一句话总结

这是一个给学生党准备的“小查词器”：

- 小
- 快
- 离线
- 不吵
- 够用
