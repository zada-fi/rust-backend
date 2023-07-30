use actix_web::{web, HttpRequest, HttpResponse};
use crate::server::AppState;
use crate::db;
use rbatis::rbdc::common::decimal::Decimal;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use crate::db::tables::Project;
use rbatis::rbdc::json::Json;
use std::str::FromStr;
use rbatis::rbdc::datetime::DateTime;
use qstring::QString;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectLink {
    pub(crate) web_url: String,
    pub(crate) twitter_url: String,
    pub(crate) dc_url: String,
    pub(crate) tg_url: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    pub(crate) project_name: String,
    pub(crate) project_description: String,
    pub(crate) project_links: ProjectLink,
    pub(crate) project_title: String,
    pub(crate) project_pic_url: String,
    pub(crate) project_address: String,
    pub(crate) project_owner: String,
    pub(crate) receive_token: String,
    pub(crate) token_symbol: String,
    pub(crate) token_address: String,
    pub(crate) token_price_usd: String,
    pub(crate) start_time: String,
    pub(crate) end_time: String,
    pub(crate) raise_limit: String,
    pub(crate) purchased_min_limit: String,
    pub(crate) purchased_max_limit: String,
    pub(crate) created_time: String,
    pub(crate) last_updated_time: String,
    pub(crate) paused: bool,
    pub(crate) total_raised: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClaimableProject {
    pub(crate) project_name: String,
    pub(crate) project_address: String,
    pub(crate) token_symbol: String,
    pub(crate) claimable_amount: String,
    pub(crate) claim_start_time: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateProjectReq {
    pub(crate) project_name: String,
    pub(crate) project_description: String,
    pub(crate) project_pic_url: String,
    pub(crate) project_title: String,
    pub(crate) project_links: ProjectLink,
    pub(crate) project_owner: String,
    pub(crate) receive_token: String,
    pub(crate) token_symbol: String,
    pub(crate) token_address: String,
    pub(crate) token_price_usd: String,
    pub(crate) start_time: String,
    pub(crate) end_time: String,
    pub(crate) raise_limit: String,
    pub(crate) purchased_min_limit: String,
    pub(crate) purchased_max_limit: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateProjectReq {
    pub project_name: String,
    pub update_name: String,
    pub update_value: String,
}
pub async fn create_project(
    data: web::Data<AppState>,
    msg: web::Json<CreateProjectReq>,
) -> actix_web::Result<HttpResponse> {
    let mut rb = data.db.clone();
    let ret =  db::get_project_by_name(&rb,msg.project_name.clone())
        .await.unwrap_or(Some(Project{..Default::default()}));
    if ret.is_some() {
        let resp = BackendResponse {
            code: BackendError::InvalidParameters,
            error: Some("Project already exist".to_string()),
            data: None::<()>,
        };
        return Ok(HttpResponse::Ok().json(resp));
    }
    let db_project = Project {
        project_name: msg.project_name.clone(),
        project_description: msg.project_description.clone(),
        project_pic_url: msg.project_pic_url.clone(),
        project_title: msg.project_title.clone(),
        project_links: Json::from_str(&serde_json::to_string(&msg.project_links).unwrap()).unwrap(),
        project_address: None,
        project_owner: msg.project_owner.clone(),
        receive_token: msg.receive_token.clone(),
        token_symbol: msg.token_symbol.to_string(),
        token_address: msg.token_address.clone(),
        token_price_usd: Decimal::from_str(&msg.token_price_usd).unwrap(),
        start_time: DateTime::from_str(&msg.start_time).unwrap(),
        end_time: DateTime::from_str(&msg.end_time).unwrap(),
        raise_limit: msg.raise_limit.clone(),
        purchased_min_limit: msg.purchased_min_limit.clone(),
        purchased_max_limit: msg.purchased_max_limit.clone(),
        created_time: DateTime::now(),
        last_updated_time: None,
        paused: false
    };
    match db::save_project(&mut rb,&db_project).await {
        Ok(()) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            println!("save project failed {:?}",e);
            let resp = BackendResponse {
                code: BackendError::InvalidParameters,
                error: Some("save project failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}
pub async fn update_project(
    data: web::Data<AppState>,
    msg: web::Json<UpdateProjectReq>,
) -> actix_web::Result<HttpResponse> {
    let mut rb = data.db.clone();
    let old_project = db::get_project_by_name(&rb,msg.project_name.clone())
        .await.unwrap_or_default();
    if old_project.is_none() {
        let resp = BackendResponse {
            code: BackendError::InvalidParameters,
            error: Some("project name not found".to_string()),
            data: None::<()>,
        };
        return Ok(HttpResponse::Ok().json(resp));
    }
    let old_project = old_project.unwrap();
    let links_str = &old_project.project_links.0[..];
    let old_project_links: serde_json::Value = serde_json::from_str(
        links_str.trim_start_matches('"')
            .trim_end_matches('"')).unwrap();
    let mut new_project = match msg.update_name.as_str() {
        "description" => {
            Project {
                project_description: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "links" => {
            Project {
                project_links: Json::from_str(&msg.update_value).unwrap(),
                ..old_project.clone()
            }
        },
        "pic_url" => {
            Project {
                project_pic_url: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "title" => {
            Project {
                project_title: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "max_cap" => {
            Project {
                raise_limit: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "user_max_cap" => {
            Project {
                purchased_max_limit: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "user_min_cap" => {
            Project {
                purchased_min_limit: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "start_time" => {
            Project {
                start_time: DateTime::from_str(&msg.update_value).unwrap(),
                ..old_project.clone()
            }
        },
        "end_time" => {
            Project {
                end_time: DateTime::from_str(&msg.update_value).unwrap(),
                ..old_project.clone()
            }
        },
        "owner" => {
            Project {
                project_owner: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "receive_token" => {
            Project {
                receive_token: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "token_symbol" => {
            Project {
                token_symbol: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "token_address" => {
            Project {
                token_address: msg.update_value.clone(),
                ..old_project.clone()
            }
        },
        "token_price" => {
            Project {
                token_price_usd: Decimal::from_str(&msg.update_value).unwrap(),
                ..old_project.clone()
            }
        },
        "pause" => {
            Project {
                paused: true,
                ..old_project.clone()
            }
        },
        "unpause" => {
            Project {
                paused: false,
                project_links: Json::from_str(&serde_json::to_string(&old_project.project_links).unwrap()).unwrap(),
                ..old_project.clone()
            }
        },
        _ => {
            old_project
        }
    };
    if msg.update_name != "links" {
        new_project.project_links = serde_json::from_value(old_project_links).unwrap();
    };
    new_project.last_updated_time = Some(DateTime::now());
    match Project::update_by_column(&mut rb,&new_project,"project_name").await {
        Ok(_) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            println!("{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("update project failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }
}

pub async fn get_all_projects(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("1").parse::<i32>().unwrap();
    match db::get_projects_by_page_number(&rb,pg_no).await {
        Ok((pg_count,projects)) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some((pg_count,projects))
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            println!("get_projects from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get projects failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}

pub async fn get_launchpad_stat_info(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_launchpad_stat_info(&rb).await {
        Ok(stat_info) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(stat_info)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_launchpad_stat_info from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get launchpad stat info failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}


pub async fn get_all_claimable_tokens(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let user_addr = match qs.get("address") {
        Some(addr) => addr.to_owned(),
        None => {
            let resp = BackendResponse {
                code: BackendError::InvalidParameters,
                error: Some("Not input address".to_string()),
                data: None::<()>,
            };
            return Ok(HttpResponse::Ok().json(resp));
        }
    };
    let pg_no = qs.get("pg_no").unwrap_or("1").parse::<i32>().unwrap();
    let user_addr = user_addr.trim_start_matches("0x").to_ascii_lowercase();
    match db::get_claimable_tokens_by_page_number(&rb,pg_no,user_addr.to_string()).await {
        Ok((pg_count,projects)) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some((pg_count,projects))
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            println!("get_claimable_projects from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_claimable_projects failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}