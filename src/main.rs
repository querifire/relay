mod local_proxy;
mod proxy_cache;
mod proxy_type;
mod sources;
mod speed_test;
mod upstream;

use anyhow::{anyhow, Result};
use clap::Parser;
use parking_lot::RwLock;
use proxy_type::{Proxy, ProxyProtocol};
use speed_test::ProxyWithSpeed;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "autoproxy")]
#[command(about = "SOCKS5 прокси-сервер с автоматическим выбором самого быстрого прокси", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "9051", help = "Порт для локального SOCKS5 сервера")]
    port: u16,

    #[arg(short, long, default_value = "127.0.0.1", help = "IP адрес для привязки")]
    bind: String,

    #[arg(short = 'u', long, help = "Upstream прокси (формат: protocol://host:port). Если не указан - автоматический поиск SOCKS5")]
    upstream: Option<String>,

    #[arg(short, long, default_value = "100", help = "Количество одновременных тестов")]
    concurrency: usize,

    #[arg(short = 'a', long, help = "Автоматическая смена прокси каждые N минут (0 = отключено)")]
    auto_rotate: Option<u64>,

    #[arg(short, long, help = "Подробный вывод логов")]
    verbose: bool,
}

fn parse_upstream_proxy(upstream: &str) -> Result<Proxy> {
    if !upstream.contains("://") {
        return Err(anyhow!("Неверный формат upstream прокси. Используйте: protocol://host:port"));
    }

    let parts: Vec<&str> = upstream.split("://").collect();
    if parts.len() != 2 {
        return Err(anyhow!("Неверный формат upstream прокси"));
    }

    let protocol = match parts[0].to_lowercase().as_str() {
        "http" | "https" => ProxyProtocol::Http,
        "socks4" => ProxyProtocol::Socks4,
        "socks5" => ProxyProtocol::Socks5,
        _ => return Err(anyhow!("Неподдерживаемый протокол: {}. Используйте: http, https, socks4, socks5", parts[0])),
    };

    let host_port: Vec<&str> = parts[1].split(':').collect();
    if host_port.len() != 2 {
        return Err(anyhow!("Неверный формат host:port"));
    }

    let host = host_port[0].to_string();
    let port = host_port[1].parse::<u16>()
        .map_err(|_| anyhow!("Неверный номер порта"))?;

    Ok(Proxy::new(host, port, protocol))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(false)
        .without_time()
        .with_ansi(false)
        .init();

    tracing::info!("[*] AutoProxy запускается...");

    let (upstream_proxy, working_proxies) = if let Some(upstream_str) = args.upstream {
        let proxy = match parse_upstream_proxy(&upstream_str) {
            Ok(proxy) => {
                tracing::info!("[*] Ручной режим - Upstream прокси: {}", proxy);
                proxy
            }
            Err(e) => {
                tracing::error!("[x] Ошибка парсинга upstream прокси: {}", e);
                tracing::info!("Примеры:");
                tracing::info!("  autoproxy -u http://1.2.3.4:8080");
                tracing::info!("  autoproxy -u socks5://1.2.3.4:1080");
                return Err(e);
            }
        };
        (proxy, Vec::new())
    } else {
        tracing::info!("[*] Автоматический режим - поиск самого быстрого SOCKS5 прокси...");
        
        tracing::info!("[*] Проверка кэша...");
        let cached_proxies = proxy_cache::load_cache().await.unwrap_or_default();
        
        let mut tested_proxies = if !cached_proxies.is_empty() {
            tracing::info!("[*] Проверка {} прокси из кэша...", cached_proxies.len());
            speed_test::test_proxies_parallel(cached_proxies.clone(), args.concurrency).await
        } else {
            Vec::new()
        };

        tracing::info!("[*] Загрузка новых списков SOCKS5 прокси...");
        let new_proxies = sources::fetch_socks5_proxies().await?;

        if new_proxies.is_empty() && tested_proxies.is_empty() {
            tracing::error!("[x] Не удалось получить ни одного прокси");
            return Err(anyhow!("Нет доступных прокси"));
        }

        let cached_set: HashSet<_> = cached_proxies.into_iter().collect();
        let new_unique: Vec<_> = new_proxies
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|p| !cached_set.contains(p))
            .collect();

        if !new_unique.is_empty() {
            tracing::info!(
                "[*] Найдено {} новых прокси, тестирование...",
                new_unique.len()
            );
            let new_tested = speed_test::test_proxies_parallel(new_unique, args.concurrency).await;
            tested_proxies.extend(new_tested);
        }

        if tested_proxies.is_empty() {
            tracing::error!("[x] Все прокси недоступны");
            return Err(anyhow!("Нет рабочих прокси"));
        }

        tracing::info!(
            "[OK] Рабочих прокси: {} из протестированных",
            tested_proxies.len()
        );

        for (i, proxy_info) in tested_proxies.iter().take(5).enumerate() {
            tracing::info!(
                "  {}. {} - {}ms",
                i + 1,
                proxy_info.proxy,
                proxy_info.latency.as_millis()
            );
        }

        let fastest = speed_test::select_fastest(tested_proxies.clone())
            .expect("Должен быть хотя бы один рабочий прокси");

        tracing::info!("[*] Выбран самый быстрый прокси:");
        tracing::info!(
            "   {} (задержка: {}ms)",
            fastest.proxy,
            fastest.latency.as_millis()
        );

        let all_working: Vec<_> = tested_proxies.iter().map(|p| p.proxy.clone()).collect();
        if let Err(e) = proxy_cache::save_cache(&all_working).await {
            tracing::warn!("[!] Не удалось сохранить кэш: {}", e);
        } else {
            tracing::info!("[*] Сохранено {} рабочих прокси в кэш", all_working.len());
        }

        (fastest.proxy, tested_proxies)
    };
    
    let bind_addr = format!("{}:{}", args.bind, args.port);
    tracing::info!("[*] Локальный SOCKS5 сервер: {}", bind_addr);

    if !working_proxies.is_empty() {
        tracing::info!("");
        tracing::info!("[*] Нажмите Enter дважды для смены прокси");
        tracing::info!("   (будут перепроверены {} рабочих прокси)", working_proxies.len());
        
        if let Some(minutes) = args.auto_rotate {
            if minutes > 0 {
                tracing::info!("[*] Автосмена прокси каждые {} минут", minutes);
            }
        }
    }

    let upstream_proxy = Arc::new(RwLock::new(upstream_proxy));
    let working_proxies = Arc::new(RwLock::new(working_proxies));
    let used_history = Arc::new(RwLock::new(VecDeque::<Proxy>::new()));

    let upstream_clone = upstream_proxy.clone();
    let working_clone = working_proxies.clone();
    let history_clone = used_history.clone();
    let concurrency = args.concurrency;
    let auto_rotate_minutes = args.auto_rotate;

    if !working_clone.read().is_empty() {
        tokio::spawn(async move {
            handle_proxy_rotation(
                upstream_clone, 
                working_clone, 
                history_clone,
                concurrency,
                auto_rotate_minutes
            ).await;
        });
    }

    local_proxy::run_socks5_server(bind_addr, upstream_proxy).await?;

    Ok(())
}

async fn handle_proxy_rotation(
    upstream_proxy: Arc<RwLock<Proxy>>,
    working_proxies: Arc<RwLock<Vec<ProxyWithSpeed>>>,
    used_history: Arc<RwLock<VecDeque<Proxy>>>,
    concurrency: usize,
    auto_rotate_minutes: Option<u64>,
) {
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    let mut last_press = std::time::Instant::now();
    let mut last_auto_rotate = std::time::Instant::now();
    
    let auto_rotate_interval = auto_rotate_minutes
        .filter(|&m| m > 0)
        .map(|m| Duration::from_secs(m * 60));

    loop {
        let check_timeout = if auto_rotate_interval.is_some() {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(60)
        };

        if let Some(interval) = auto_rotate_interval {
            let elapsed = last_auto_rotate.elapsed();
            if elapsed >= interval {
                tracing::info!("");
                tracing::info!("[*] Автоматическая смена прокси (прошло {} минут)", elapsed.as_secs() / 60);
                
                if perform_proxy_rotation(
                    &upstream_proxy,
                    &working_proxies,
                    &used_history,
                    concurrency,
                    true
                ).await {
                    last_auto_rotate = std::time::Instant::now();
                }
            }
        }

        match tokio::time::timeout(check_timeout, lines.next_line()).await {
            Ok(Ok(Some(_))) => {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_press);
                
                if elapsed < Duration::from_millis(500) {
                    tracing::info!("");
                    tracing::info!("[*] Ручная смена прокси...");
                    
                    perform_proxy_rotation(
                        &upstream_proxy,
                        &working_proxies,
                        &used_history,
                        concurrency,
                        false
                    ).await;
                }
                
                last_press = now;
            }
            Ok(Ok(None)) => break,
            Ok(Err(_)) => break,
            Err(_) => {}
        }
    }
}

async fn perform_proxy_rotation(
    upstream_proxy: &Arc<RwLock<Proxy>>,
    working_proxies: &Arc<RwLock<Vec<ProxyWithSpeed>>>,
    used_history: &Arc<RwLock<VecDeque<Proxy>>>,
    concurrency: usize,
    is_auto: bool,
) -> bool {
    let current_proxy = upstream_proxy.read().clone();
    let proxies_to_test = {
        let working = working_proxies.read();
        working.iter().map(|p| p.proxy.clone()).collect::<Vec<_>>()
    };

    if proxies_to_test.len() < 2 {
        tracing::warn!("[!] Недостаточно рабочих прокси для смены");
        return false;
    }

    tracing::info!("[*] Перепроверка {} рабочих прокси...", proxies_to_test.len());
    
    let tested = speed_test::test_proxies_parallel(proxies_to_test, concurrency).await;
    
    if tested.is_empty() {
        tracing::error!("[x] Все прокси стали недоступны");
        return false;
    }

    *working_proxies.write() = tested.clone();

    let history = used_history.read();
    let history_set: HashSet<_> = history.iter().collect();
    
    let new_proxy = tested.iter()
        .find(|p| p.proxy != current_proxy && !history_set.contains(&p.proxy))
        .or_else(|| tested.iter().find(|p| p.proxy != current_proxy))
        .or_else(|| tested.first());

    drop(history);

    if let Some(new_proxy_info) = new_proxy {
        *upstream_proxy.write() = new_proxy_info.proxy.clone();
        
        {
            let mut history = used_history.write();
            history.push_back(current_proxy.clone());
            if history.len() > 10 {
                history.pop_front();
            }
        }
        
        let rotation_type = if is_auto { "Автоматическая" } else { "Ручная" };
        tracing::info!("[OK] {} смена прокси выполнена:", rotation_type);
        tracing::info!("   {} (задержка: {}ms)", 
            new_proxy_info.proxy, 
            new_proxy_info.latency.as_millis());
        
        let history_len = used_history.read().len();
        tracing::info!("   История использования: {} из 10", history_len);
        
        if !is_auto {
            tracing::info!("");
            tracing::info!("[!] Внимание: активные SSH/длительные соединения могут разорваться!");
            tracing::info!("[*] Нажмите Enter дважды для новой смены");
        }
        
        true
    } else {
        false
    }
}
