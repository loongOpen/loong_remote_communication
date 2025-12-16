1. 完善 portal hub 实现，提供 grpc 和 restful 接口
2. 使用 Cargo features 来控制：可选编译 grpc 的可执行文件
3. 通过 builder 工厂模式来实现封装 PortalManager 和 proxyManager 的 event 逻辑，仅返回一个 event_loop 供外部 tokio::select!
4. 有一个来自 webrtc_sctp 的库 error，并不影响使用，日志屏蔽掉。**[DISPUTED: 检查是否有资源泄露]**
5. src/common.rs 统一命令行 -> mqcfg 逻辑；统一初始化代码；增加 peercfg 的命令行参数；可以传入多个 --stun turn
6. docker compose 增加 debug 编译，release 编译时长太久；docker compose 增加测试用例
7. 完善测试用例，每个 `crates/*/tests` 目录

---

1. binder 完善日志；完善关闭逻辑，dc 和 socket 生命周期保持一致，避免资源泄露
2. Portal 异常断开时，proxy 被通知具有滞后性，Portal 立马重连会导致逻辑上 bug： 为 proxy 实现 drop，在 close 时，proxyManager 删除 proxy 增加筛选条件，pc 状态 unactivate 才删除
3. 完善日志，注释，清理，记录操作后 Manager 容器的当前长度
4. 按照 portal PortalManager 的设计重构 proxy proxyManager
5. 【portal】 使用 Unix 和 Tcp 共有的 trait 来统一 accept_loop，精简代码；unix 文件清理；日志完善
6. 【PortalManager】arc portal_event_rx 在 PortalManager 中应该是独占，但他的方法都是 mut ，如果用 self.portal_event_rx 会导致 mut 传递，导致 self 没法 arc。rust Arc 只能提供 &self，不能提供 &mut self。通过引入一个 run 函数解决，在 run 函数中将 portal_event_rx 作为 mut 传入，所有权转移给 run。**[DISPUTED: 后续可通过 builder 工厂模式来实现封装]**。rust 设计哲学：狠狠地明确所有权，哪怕是微小的一个成员字段
7. 【peer】portal 持有 arc signal，删除 PortalEvent 不合理：当其 close 时无法通知 PortalManager 删除它。重新设计：利用 mpsc，PortalManager 持有唯一的 event_rx，其管理的 Portal 持有 arc event_tx ，利用一个 mpsc 让 Manager 统一管理所有的 Portal 的事件，然后通过 tokio select 可统一处理 signal 的 event 和 Portal 的 event。这样 Portal 还不用持有 signal 的 arc；完美，沃日
8. 【portal】保持代码的简洁，不要为了预留重连逻辑使用复杂的（linus 原则）pc: Arc<RwLock<Option<Arc<RTCPeerConnection>>>，abort_handles 没必要加锁，改为 pc: Arc<RTCPeerConnection>,
9. 【peer】portal 持有 arc signal，去掉 PortalEvent
10. 【PortalManager】PortalManager 和 Portal 的职责区分，addr uri 应该放在 Portal 中，事件循环当 Portal drop 时 abort
11. 【PortalManager】PortalManager 的生命周期应该和 signal 一致，signal 退出时，Manager 则不可用 应该退出或者 drop

---

1. 【signal】完善 message config topic,添加[derive(Default)] ,，引入 strum 简化
2. 【signal】thiserror 对比 anyhow ，简化错误处理，仅做错误打印，anyhow 更合适
3. 【signal】start*event_loop 日志完善，增加跳出原因；使用 let * = tx.send(x) 忽略事件发送失败 **[DISPUTED: 如果发送失败咋办！小概率，除非库 bug]**
   - 实现顶掉时的通知机制同样复杂
   - MQTT 协议不支持拒绝新连接，硬要实现比较复杂，于是**顶掉旧连接**
   - rumqttc 有内置的自动重连机制会导致反复 踢掉-重连
4. 同名的 mqtt broker 会踢掉前一个，signal event_loop 增加 break 条件：被踢掉，或者发布 online 失败，或者 subscribe signal topic 失败
5. 删除 signal Drop 方法，因为已经有 遗嘱消息
6. docker build 改为使用 user 权限，不然 vscode 插件无法生效
7. 提高 signal crate 的代码质量：类名重命名，方法重命令，引入 strum 简化 enum str
8. 增加命令行参数，干掉代码中硬编码的配置项
9. 增加交叉编译选项，文档，docker compose
10. 重构 workspace dependencie 还重构所有 crates 下的 Cargo.toml 。只启用必要特性，只写用到的库，Cargo.toml，README.md
