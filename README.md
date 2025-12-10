# kong-ics

## pm2 で approve_manager を常駐させる手順

1. ビルドと環境準備  
   - `cd /root/kong-ics`  
   - `cargo build --release`  
   - `.env` を同ディレクトリに配置（署名鍵パスや webhook を設定）

2. pm2 で起動（Rust バイナリを直接実行）  
   - `cd /root/kong-ics`  
   - `pm2 start ./target/release/approve_manager --name approve --interpreter none`  
   - `pm2 save`  
   - `pm2 startup` （表示されるコマンドを実行して自動起動を有効化）

3. ログ確認・再起動  
   - `pm2 logs approve`  
   - `pm2 restart approve`

4. 更新時の手順  
   - `cd /root/kong-ics`  
   - `git pull`  
   - `cargo build --release`  
   - `pm2 restart approve`
