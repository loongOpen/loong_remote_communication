1. [x] 日志，开始调用 grpc 服务时的日志；proxy 被多个连接时的处理
2. [x] webrtc_sctp 的 error 日志，但是不影响使用，经过测试也不导致资源泄露
3. [x] 编译选项设置 grpc 为可选编译
4. [x] PortalManager 和 proxy_manager 的自包含性，应该不要对外暴露复杂的 run 方法和 event loop
5. [x] Portal hub 提供 grpc 和 http 实现，通过 api 创建
6. [x] mdbook 生成文档，结合 github action 自动发布 github pages
7. [ ] 完善使用文档，设计架构图
8. [ ] Portal::new 时应该 local_id 用 ref ，config 也用 ref ，减少拷贝
9. [ ] 在 PortalManager 或者 Portal 中实现重连接
10. [ ] 一 protal 对多 proxy 转发，集群控制
11. [ ] 搞一个 peer.rs 他的 核心是一个 peer 的 trait 和 PeerEvent ，来抽象 protal 和 proxy
12. [ ] build so for android，usage， docker，use in android
13. [ ] 实现 https 代理
