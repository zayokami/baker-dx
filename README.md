# Baker

基于 Dioxus 0.7 的会话编辑与回放界面，面向聊天场景的 UI 原型与交互演示。
这个项目旨在还原《明日方舟：终末地》中 Baker 的聊天功能，当然还有一点功能没实现的……留作以后再完善。

## 功能概览

- 会话与联系人管理：选择会话、发起单聊或群聊、配置干员列表
- 消息编辑能力：发送、编辑、删除、在指定位置插入
- 消息类型支持：普通消息、状态行、图片、贴纸
- 反应与演出：消息反应、发送动画、回放打字效果
- 回放能力：从指定消息起开始回放，并在回放结束后显示“话题结束”
- 导出能力：离屏渲染当前会话并导出截图
- 个性化设置：会话头样式切换、背景模式设置、用户资料配置、教程开关
- 本地持久化存储：当前版本使用 LocalStorage + IndexedDB，并兼容旧版 `baker_dx_state.json` 数据迁移

## TO-DO LIST

- [ ] 移动端有限宽度的布局支持
- [ ] 应用配置导出
- [ ] “任务”消息支持
- [x] 会话的离屏渲染并导出
- [ ] 群组会话时控制哪些干员消息位于会话右方
- [x] 回放结束后显示“话题结束，暂无新话题”的提示

## 使用的技术

- Rust 2024
- Dioxus 0.7.3
- 桌面端默认启用，Web 端可选
- Dioxus Router
- `reqwest` / `webbrowser` / `image`

## 运行

```bash
cargo install dioxus-cli
dx serve --platform desktop
dx serve --platform web
```

默认特性为 `desktop`。如果你希望只构建 Web，可以按需调整 Cargo feature。

## 存储说明

- 当前状态数据会序列化为 v2 存储结构
- 元数据使用 LocalStorage 保存
- 联系人、消息和图片资源使用 IndexedDB 保存
- 旧版本的本地 JSON 状态文件 `baker_dx_state.json` 仍可作为迁移来源读取

## 项目结构

- `src/main.rs`：应用入口、资源注入、状态加载与保存
- `src/components/baker/layout.rs`：主页面控制层与路由入口
- `src/components/baker/chat_area.rs`：聊天区域与消息渲染
- `src/components/baker/input_bar.rs`：输入栏、图片与贴纸发送
- `src/components/baker/modals.rs`：各类弹窗
- `src/components/baker/storage.rs`：状态编码、解码与迁移逻辑
- `server/`：独立的轻量服务端子工程

## 问题、建议、Pull Request

如果你在使用这个软件时遇到任何 bug 或者对软件有啥建议，请在 Github 上提交 Issue，谢谢！

如果有什么功能你想实现，也欢迎提交 Pull Request，我会尽快合并。

---

_……_

_我是管理员，这个项目的作者请我帮忙带个话：“**请注意，作者在天师考校上课，由于天师考校复课（高中开学），因此作者在工作日可能不能及时回复问题或者查看拉取请求。**”谢谢你的理解！_

## 展示

![展示-Perlica](./readme/perlica.png)
