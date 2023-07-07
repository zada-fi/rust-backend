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
    web_url: String,
    twitter_url: String,
    dc_url: String,
    tg_url: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    pub(crate) project_name: String,
    pub(crate) project_description: String,
    pub(crate) project_links: ProjectLink,
    pub(crate) project_owner: String,
    pub(crate) receive_token: String,
    pub(crate) token_address: String,
    pub(crate) token_price_usd: String,
    pub(crate) start_time: String,
    pub(crate) end_time: String,
    pub(crate) raise_limit: i32,
    pub(crate) purchased_min_limit: i32,
    pub(crate) purchased_max_limit: i32,
    pub(crate) created_time: String,
    pub(crate) last_updated_time: String,
    pub(crate) paused: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateProjectReq {
    pub project_name: String,
    pub update_name: String,
    pub update_value: String,
}
pub async fn create_project(
    data: web::Data<AppState>,
    msg: web::Json<ProjectInfo>,
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
        project_links: Json::from_str(&serde_json::to_string(&msg.project_links).unwrap()).unwrap(),
        project_owner: msg.project_owner.clone(),
        receive_token: msg.receive_token.clone(),
        token_address: msg.token_address.clone(),
        token_price_usd: Decimal::from_str(&msg.token_price_usd).unwrap(),
        start_time: DateTime::from_str(&msg.start_time).unwrap(),
        end_time: DateTime::from_str(&msg.end_time).unwrap(),
        raise_limit: msg.raise_limit,
        purchased_min_limit: msg.purchased_min_limit,
        purchased_max_limit: msg.purchased_max_limit,
        created_time: DateTime::from_str(&msg.created_time).unwrap(),
        last_update_time: DateTime::from_str(&msg.last_updated_time).unwrap(),
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
    let new_project = match msg.update_name.as_str() {
        "max_cap" => {
            Project {
                raise_limit: msg.update_value.parse::<i32>().unwrap(),
                ..old_project.clone()
            }
        },
        "user_max_cap" => {
            Project {
                purchased_max_limit: msg.update_value.parse::<i32>().unwrap(),
                ..old_project.clone()
            }
        },
        "user_min_cap" => {
            Project {
                purchased_min_limit: msg.update_value.parse::<i32>().unwrap(),
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
                ..old_project.clone()
            }
        },
        _ => {
            old_project
        }
    };

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
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("update  project failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }
}


//
// function updateProjectOwner(address project,address newOwner) public onlyGovernor {
// IProject(project).updateProjectOwner(newOwner);
// }
//
// function pause(address project) public onlyGovernor {
// IProject(project).pause();
// }
//
// function unpause(address project) public onlyGovernor {
// IProject(project).unpause();
// }
pub async fn get_all_projects(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("0").parse::<i32>().unwrap();
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
            log::warn!("get_projects from db failed,{:?}",e);
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
    req: HttpRequest,
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
