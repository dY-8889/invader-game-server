use std::{
    fmt::Display,
    io::Write,
    net::{IpAddr, TcpStream},
    sync::Mutex,
};

use actix_web::{
    post,
    web::{Data, Json, Path},
    App, HttpResponse, HttpServer, Responder,
};
use serde::{Deserialize, Serialize};

struct RoomList(Mutex<Vec<Room>>);

// TODO: HashMapにする
#[derive(Debug)]
struct WaitRoomList(Mutex<Vec<RoomRequest>>);

#[derive(Debug)]
struct Room {
    _id: u32,
    _user1: User,
    _user2: User,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RoomRequest {
    room_id: u32,
    user: User,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "ip")]
    _ip: IpAddr,
    #[serde(rename = "delta_seconds")]
    _delta_seconds: f32,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let split = self._ip.to_string();
        let split: Vec<&str> = split.split('.').collect();

        let mut ip: [u8; 4] = [0, 0, 0, 0];
        for (i, str) in split.iter().enumerate() {
            ip[i] = str.trim().parse().unwrap();
        }

        write!(
            f,
            "
{{
    \"name\" : \"{}\",
    \"ip\" : {:?},
    \"delta_seconds\" : {} 
}}",
            self._name, ip, self._delta_seconds
        )
    }
}

impl Room {
    fn new(_id: u32, _user1: User, _user2: User) -> Room {
        Room {
            _id,
            _user1,
            _user2,
        }
    }
}

#[derive(Serialize)]
enum ResultResponse {
    Ok { message: String, user: Option<User> },
    Err(String),
}

#[post("/create")]
async fn create(request: Json<RoomRequest>, wait_room: Data<WaitRoomList>) -> impl Responder {
    let mut wait_room = wait_room.0.lock().unwrap();

    for room in wait_room.iter_mut() {
        if room.room_id == request.room_id {
            return HttpResponse::Ok().json(ResultResponse::Err(
                "このidの待ち部屋はすでにあります".to_string(),
            ));
        }
    }
    wait_room.push(request.0);

    println!("create wait room: {:?}", wait_room);

    HttpResponse::Ok().json(ResultResponse::Ok {
        message: "待ち部屋の作成に成功しました".to_string(),
        user: None,
    })
}

#[post("/enter")]
async fn enter(
    request: Json<RoomRequest>,
    wait_room: Data<WaitRoomList>,
    room_list: Data<RoomList>,
) -> impl Responder {
    // 入りたい部屋
    let request = request.0;
    // 待ち部屋list
    let mut wait_room_list = wait_room.0.lock().unwrap();

    for (i, wait_room) in wait_room_list.iter_mut().enumerate() {
        // 待ち部屋と入りたい部屋のidが一致するなら
        if wait_room.room_id == request.room_id {
            // 新しい部屋を作る
            let new_room = Room::new(
                wait_room.room_id,
                wait_room.user.clone(),
                request.user.clone(),
            );
            println!("new room!: {:#?}", new_room);

            // 待ち部屋のユーザーの情報を入ってきたユーザーに送る
            let user = wait_room.user.clone();
            // ストリームを得る(???)
            let mut stream = TcpStream::connect(user._ip.to_string() + ":8888").unwrap();
            if let Err(e) = stream.write_all(request.user.to_string().as_bytes()) {
                println!("stream err: {:?}", e);
            }

            // 部屋のリストに新しい部屋を追加
            room_list.0.lock().unwrap().push(new_room);
            // 待ち部屋を削除
            wait_room_list.remove(i);

            // リクエストを送ってきたユーザーに待ち部屋のユーザーの情報と
            // 入室に成功したことをResponseする
            return HttpResponse::Ok().json(ResultResponse::Ok {
                message: "部屋に入りました".to_string(),
                user: Some(user),
            });
        }
    }

    HttpResponse::Ok().json(ResultResponse::Err(
        "入る部屋がありませんでした".to_string(),
    ))
}

#[post("/delete/room/{index}")]
async fn delete(index: Path<usize>, wait_room: Data<WaitRoomList>) -> impl Responder {
    let mut wait_room = wait_room.0.lock().unwrap();
    wait_room.remove(index.into_inner());

    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let room_list = Data::new(RoomList(Mutex::new(vec![])));
    let wait_room = Data::new(WaitRoomList(Mutex::new(vec![])));
    HttpServer::new(move || {
        App::new()
            .service(create)
            .service(enter)
            .app_data(room_list.clone())
            .app_data(wait_room.clone())
    })
    .bind(("192.168.116.115", 9999))?
    .run()
    .await?;

    Ok(())
}
