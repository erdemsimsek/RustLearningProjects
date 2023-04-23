use serde::Deserialize;
use enum_iterator::{all, cardinality, Sequence};
use eframe::egui::{self, RichText};
use std::{thread, time, error::Error, sync::mpsc, env};

#[derive(PartialEq)]
struct NewsReportConfig{
    selected_categories : Vec<String>,
    polling_interval : u64,
}

#[derive(Deserialize, Debug)]
struct Articles {
    articles: Vec<Article>
}

#[derive(Deserialize, Debug)]
struct Article {
    title : String,
    url : String,
}

#[derive(Sequence, Debug)]
enum Categories{
    Business,
    Entertainment,
    General,
    Health,
    Science,
    Sports,
    Technology,
}

impl Categories {
    fn to_string(&self) -> String {
        match self {
            Categories::Business => "Business",
            Categories::Entertainment => "Entertainment",
            Categories::General => "General",
            Categories::Health => "Health",
            Categories::Science => "Science",
            Categories::Sports => "Sports",
            Categories::Technology => "Technology",
        }.to_string()
    }
}


fn get_articles( list_of_desired_categories : Vec<String> ) -> Result<Articles, Box<dyn Error>>{
    let mut api_key = String::new();
    match env::var("NEWS_API_KEY") {
        Ok(key) => api_key = key,
        Err(e) => panic!("News API key couldn't be found in the environment")
    }

    let mut list_of_categories_with_formatted = String::new();

    // Convert this to a lambda function
    for category in list_of_desired_categories {
        list_of_categories_with_formatted.push_str(&format!("&category={}", category));
    }

    let query_addr = format!("https://newsapi.org/v2/top-headlines?country=gb{}&apiKey={}", list_of_categories_with_formatted, api_key);
    let response = ureq::get(query_addr.as_str()).call()?.into_string()?;
    let articles : Articles = serde_json::from_str(&response)?;
    Ok(articles)
}


// GUI definitions

struct NewsReports{
   category_flag : [bool; cardinality::<Categories>()],
   polling_interval : u64,
   channel_to_news : mpsc::Sender<NewsReportConfig>,
   channel_to_gui : mpsc::Receiver<Articles>,
   list_of_articles : Vec<Article>
}

impl eframe::App for NewsReports {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            
            show_menu(ui, &mut self.category_flag, &mut self.polling_interval, &mut self.channel_to_news);
            
            ui.heading("News Reports");
            match self.channel_to_gui.try_recv() {
                Ok(articles) => {
                    self.list_of_articles = articles.articles;
                },
                _ => {}
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                for article in self.list_of_articles.iter(){
                    ui.label(RichText::new(format!("{}", article.title)).color(egui::Color32::from_rgb(173, 216, 230)));
                    ui.hyperlink(article.url.to_string()).on_hover_text("Click to see the news");
                    ui.separator();
                }
            });

        });
    }
}

fn show_menu(ui: &mut egui::Ui, category_flag: &mut [bool; cardinality::<Categories>()], polling_interval: &mut u64, channel_to_news: &mut mpsc::Sender<NewsReportConfig>){

        use egui::{menu};

        menu::bar(ui, |ui| {
            ui.menu_button("Settings", |ui| {
                let mut is_update_required = false;
    
                // Put items inside lambda to avoid creating them if the menu is not open
                ui.menu_button("Categories", |ui: &mut egui::Ui| {
                
                    let mut enum_counter = 0;
                    let mut selected_categories = category_flag.clone();
                    for category in all::<Categories>(){
                        ui.checkbox(&mut selected_categories[enum_counter], category.to_string());
                        enum_counter += 1;
                    }
                    if selected_categories != *category_flag {
                        *category_flag = selected_categories;
                        is_update_required = true;
                    }
                });

                ui.menu_button( "Refresh speed", |ui: &mut egui::Ui| {
                    let mut requested_polling_rate = *polling_interval;
                    ui.add(
                        egui::Slider::new(&mut requested_polling_rate, 60..=3600).text("sec").logarithmic(true)
                    );
                    if requested_polling_rate != *polling_interval {
                        *polling_interval = requested_polling_rate;
                        is_update_required = true;
                    }
                });

                if is_update_required {
                    let mut counter = 0;
                    let mut news_config = NewsReportConfig{
                        selected_categories : vec![],
                        polling_interval : *polling_interval,
                    };
                    for category in all::<Categories>(){
                        if category_flag[counter] {
                            news_config.selected_categories.push(category.to_string());
                        }
                        counter += 1;
                    }
                    channel_to_news.send(news_config.into()).unwrap();
                    is_update_required = false;
                }
    
            });
        
        });
    }

fn main() {

    let (from_gui, to_news) = mpsc::channel::<NewsReportConfig>();
    let (from_news, to_gui) = mpsc::channel::<Articles>();

    let news_thread_handle = thread::spawn(move || {
        
        let mut default_news_config = NewsReportConfig{
            selected_categories : vec![],
            polling_interval : 1,
        };

        loop {
            match to_news.try_recv() {
                Ok(received_message) => {
                    default_news_config = received_message;
                },
                _ => {}
            }

            let articles = get_articles( default_news_config.selected_categories.clone() );
 
            from_news.send(articles.unwrap()).unwrap();

            thread::sleep(time::Duration::from_secs(default_news_config.polling_interval));
        }
    });

    let options = eframe::NativeOptions {
        transparent: false,
        initial_window_size: Some(egui::vec2(400.0, 300.0)),
        min_window_size: Some(egui::vec2(400.0, 300.0)),
        ..Default::default()
    };


    let app = NewsReports{
        category_flag: ([true;7]),
        polling_interval: 600,
        channel_to_news: from_gui.clone(),
        channel_to_gui: to_gui,
        list_of_articles: vec![]
    };

    eframe::run_native(
        "News Reports",
        options,
        Box::new(|_cc: &eframe::CreationContext| Box::<NewsReports>::new(app)),
    );

    let _res = news_thread_handle.join();

}


