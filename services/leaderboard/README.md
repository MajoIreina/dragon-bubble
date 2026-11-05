# 预留：匿名定位练习排行榜

**0.2.1 不启用、不部署、不在客户端显示，不发出排行榜请求。**

客户端入口位于 `crates/pet-desktop/src/leaderboard.rs`，由 `ENABLED = false` 封闭。将来准备可信 HTTPS 服务后，再设置发布构建环境 `LPP_LEADERBOARD_URL=https://<域名>/dragon-bubble/v1` 并开启功能。玩家只填写昵称，无需 GitHub 登录；客户端需明确展示公开成绩范围并由玩家启用自动提交。

## 接口契约

- `GET /dragon-bubble/v1/health`：`{"ok":true,"protocol":1}`。
- `GET /dragon-bubble/v1/scores`：最多 100 条 `[{"nickname":"昵称","hits":30,"timestamp":1730000000}]`，按最高分降序，同昵称保留最佳。
- `POST /dragon-bubble/v1/scores`：`{"protocol":1,"nickname":"昵称","record":{"id":"32位小写十六进制ID","timestamp":1730000000,"hits":30,"screen":0,"width":1920,"height":1080}}`；返回更新后的榜单。
- 时间由服务端生成。昵称 NFKC 归一化、1–24 字且无控制字符；点击数 0–750；限制请求体 4 KiB，客户端响应 128 KiB；重复局号幂等。IP 限流只保留短时哈希，禁止记录原始请求内容。
- 成绩为自报数据，不能宣称防作弊或经过身份验证，昵称无独占权。将来启用前须补充内容管理与删除流程、独立网关限流、实际 HTTPS 和匿名用户端到端测试。

## 服务草案

`server.py` 使用 Python 标准库及服务端 Git，将数据保存到单独数据仓库。建议为该仓库单独生成可写 Deploy Key；客户端与反向代理都不持有仓库凭据。不要把开发者 PAT 或主仓库凭据复制到服务器。

服务的 `SCORES_DATA` 必须是服务专用可丢弃 Git 检出目录，`origin/main` 已存在；写入失败返回 503，客户端保留本地成绩。`SCORES_LOCAL_ONLY=1` 仅供本地接口测试，不得用于正式的 GitHub 同步服务。单实例服务串行化 Git 更新，最多 16 个请求线程并有内存上限。模板 `dragon-scores.service` 尚未部署。

**注意**：示例服务只信任来自回环地址的 `X-Real-IP`；异机反向代理上线前须限定可信代理地址并验证 IP 传递。现有主机及路由配置不属于本版发布范围。
