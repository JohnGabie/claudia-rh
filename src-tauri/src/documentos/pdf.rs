//! HTML -> PDF with the app's OWN WebView2 (the Edge engine Tauri already runs on), in a hidden window.
//! No browser of ours, no new runtime: the same HTML the user previews becomes the PDF.
use std::path::Path;
use std::time::Duration;

const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
const PRINT_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn html_to_pdf(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::print(app, html, pdf).await
    }
    #[cfg(not(windows))]
    {
        let _ = (app, html, pdf);
        Err("gerar PDF só está disponível no Windows".into())
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tauri::{webview::PageLoadEvent, WebviewUrl, WebviewWindowBuilder};
    use tokio::sync::oneshot;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment6, ICoreWebView2_7};
    use webview2_com::PrintToPdfCompletedHandler;
    use windows_core::{Interface, HSTRING};

    type Slot<T> = Arc<Mutex<Option<oneshot::Sender<T>>>>;

    fn send<T>(slot: &Slot<T>, v: T) {
        if let Some(tx) = slot.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(v);
        }
    }

    pub async fn print(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
        let url = tauri::Url::from_file_path(html).map_err(|_| "caminho do HTML inválido".to_string())?;
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let label = format!("pdf-{nanos}");
        let (loaded_tx, loaded_rx) = oneshot::channel::<()>();
        let loaded: Slot<()> = Arc::new(Mutex::new(Some(loaded_tx)));
        let loaded_cb = loaded.clone();
        // Hidden, A4-ish size, no capability for this label: the page has no IPC to the app.
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
            .visible(false)
            .inner_size(794.0, 1123.0)
            .on_page_load(move |_w, p| {
                if p.event() == PageLoadEvent::Finished {
                    send(&loaded_cb, ());
                }
            })
            .build()
            .map_err(|e| format!("não foi possível abrir a janela de impressão: {e}"))?;

        let result = async {
            tokio::time::timeout(LOAD_TIMEOUT, loaded_rx)
                .await
                .map_err(|_| "o HTML não carregou a tempo".to_string())?
                .map_err(|_| "a janela de impressão fechou".to_string())?;
            let (tx, rx) = oneshot::channel::<Result<(), String>>();
            let slot: Slot<Result<(), String>> = Arc::new(Mutex::new(Some(tx)));
            let out = HSTRING::from(pdf.as_os_str());
            let slot_wv = slot.clone();
            win.with_webview(move |wv| {
                let slot_cb = slot_wv.clone();
                let started: windows_core::Result<()> = unsafe {
                    (|| {
                        let core = wv.controller().CoreWebView2()?;
                        let core7: ICoreWebView2_7 = core.cast()?;
                        let env6: ICoreWebView2Environment6 = wv.environment().cast()?;
                        let s = env6.CreatePrintSettings()?;
                        s.SetShouldPrintBackgrounds(true)?;
                        s.SetShouldPrintHeaderAndFooter(false)?;
                        s.SetPageWidth(8.27)?; // A4, inches
                        s.SetPageHeight(11.69)?;
                        s.SetMarginTop(0.0)?;
                        s.SetMarginBottom(0.0)?;
                        s.SetMarginLeft(0.0)?;
                        s.SetMarginRight(0.0)?;
                        let handler = PrintToPdfCompletedHandler::create(Box::new(move |hr: windows_core::Result<()>, ok: bool| {
                            send(&slot_cb, match (hr, ok) {
                                (Ok(()), true) => Ok(()),
                                (Ok(()), false) => Err("o WebView2 não gerou o PDF".to_string()),
                                (Err(e), _) => Err(format!("erro do WebView2 ao gerar o PDF: {e}")),
                            });
                            Ok(())
                        }));
                        core7.PrintToPdf(&out, &s, &handler)
                    })()
                };
                if let Err(e) = started {
                    send(&slot_wv, Err(format!("não foi possível iniciar a impressão: {e}")));
                }
            })
            .map_err(|e| e.to_string())?;
            tokio::time::timeout(PRINT_TIMEOUT, rx)
                .await
                .map_err(|_| "a geração do PDF demorou demais".to_string())?
                .map_err(|_| "a impressão foi interrompida".to_string())?
        }
        .await;
        let _ = win.destroy();
        result
    }
}
