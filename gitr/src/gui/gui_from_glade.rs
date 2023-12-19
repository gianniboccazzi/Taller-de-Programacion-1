use std::fs;

use gtk::gio::ApplicationFlags;
use gtk::{prelude::*, Application, ComboBoxText, Dialog, Entry, Label, TextBuffer, TextView, ListBox, ListBoxRow, Orientation};

use gtk::{Builder, Button, FileChooserButton, Window};

use crate::commands::command_utils::_create_pr;
use crate::commands::commands_fn::{self};
use crate::file_manager;
use crate::gitr_errors::GitrError;
use crate::objects::pull_request::PullRequest;

fn get_commits(cliente: String) -> String {
    let mut commits = match file_manager::commit_log("-1".to_string(), cliente) {
        Ok(commits) => commits,
        Err(_) => return "No hay commits para mostrar".to_string(),
    };
    commits = commits.trim_end().to_string();

    let mut res = String::new();
    let max_string_len = 60;
    
    let mut fecha_actual = "-1";
    for mut commit in commits.split("\n\n\n").collect::<Vec<&str>>() {
        let corrimiento_merge = if commit.contains("Merge") {
            1
        } else {
            0
        };
        commit = commit.trim_start();
        let hash = commit.split('\n').collect::<Vec<&str>>()[corrimiento_merge].split_at(8).1;
        let author = commit.split('\n').collect::<Vec<&str>>()[1 + corrimiento_merge].split_at(7).1;
        let date = commit.split('\n').collect::<Vec<&str>>()[2 + corrimiento_merge].split_at(5).1.trim_start();
        let message = commit.split('\n').collect::<Vec<&str>>()[3 + corrimiento_merge*2].trim_start();
        
        let day = date.split(' ').collect::<Vec<&str>>()[2];
        let time = date.split(' ').collect::<Vec<&str>>()[3];
        let hash_digits = hash.split_at(8).0;
        let short_message = if message.len() > 40 {
            message[..40].to_string() + "..."
        } else {
            message.to_string()
        };

        if day != fecha_actual {
            let month = date.split(' ').collect::<Vec<&str>>()[1];
            let year = date.split(' ').collect::<Vec<&str>>()[4];
            res.push_str("█\n");
            let fecha = format!("█■■> Commits on {} {}, {}\n", month, day, year);
            res.push_str(&fecha);
            res.push_str("█\n");
        }
        fecha_actual = day;
        let spaces_needed_first_line = max_string_len - short_message.len() - hash_digits.len();
        let spaces_needed_second_line = max_string_len - author.len() - time.len() - 3;

        res.push_str("█    ╔══════════════════════════════════════════════════════════════╗\n");
        res.push_str(&format!(
            "█    ║ {}{:<width$}{} ║\n",
            short_message,
            "",
            hash_digits,
            width = spaces_needed_first_line
        ));
        res.push_str(&format!(
            "█    ║ {}    {}{:<width$}║\n",
            author,
            time,
            "",
            width = spaces_needed_second_line
        ));
        res.push_str("█    ╚══════════════════════════════════════════════════════════════╝\n");
    }
    res
}

fn email_valido(email_recibido: String) -> bool {
    let email_parts: Vec<&str> = email_recibido.split('@').collect::<Vec<&str>>();
    if email_parts.len() != 2 {
        return false;
    }

    let domain = email_parts[1];

    if !domain.contains('.') {
        return false;
    }

    true
}

fn update_branches(branch_selector: &ComboBoxText, cliente: String) {
    branch_selector.remove_all();
    let branches = match file_manager::get_branches(cliente.clone()) {
        Ok(branches) => branches,
        Err(e) => {
            println!("Error al obtener branches: {:?}", e);
            return;
        }
    };
    for branch in branches {
        branch_selector.append_text(&branch);
    }
}

fn existe_config(cliente: String) -> bool {
    fs::metadata(cliente.clone() + "/gitrconfig").is_ok()
}

fn build_ui(application: &gtk::Application, cliente: String) -> Option<String> {
    let glade_src = include_str!("gui_test.glade");
    let builder = Builder::from_string(glade_src);

    //====Builders para componentes====
    let window: Window = builder.object("main_window")?;
    let repo_selector: FileChooserButton = builder.object("repo_selector")?;
    let clone_button: Button = builder.object("clone_button")?;
    let clone_dialog: Dialog = builder.object("clone_dialog")?;
    let clone_url: Entry = builder.object("clone_url")?;
    let clone_accept_button: Button = builder.object("clone_accept_button")?;
    let commit_text: TextView = builder.object("commit_text")?;
    let branch_selector: ComboBoxText = builder.object("branch_selector")?;
    let buffer: TextBuffer = commit_text.buffer()?;
    let commit: Button = builder.object("commit_button")?;
    let commit_dialog: Dialog = builder.object("commit_dialog")?;
    let commit_confirm: Button = builder.object("confirm_commit_button")?;
    let commit_message: Entry = builder.object("commit_message")?;
    let login_dialog: Window = builder.object("login_dialog")?;
    let login_warning: Dialog = builder.object("login_warning")?;
    let connect_button: Button = builder.object("connect_button")?;
    let login_button: Button = builder.object("login_button")?;
    let mail_entry: Entry = builder.object("mail_entry")?;
    let push_button: Button = builder.object("push_button")?;
    let pull_button: Button = builder.object("pull_button")?;
    let fetch_button: Button = builder.object("fetch_button")?;
    let remote_error_dialog: Dialog = builder.object("remote_error")?;
    let remote_error_close_button: Button = builder.object("remote_error_close_button")?;
    let close_commit_dialog_button: Button = builder.object("close_commit_dialog_button")?;
    let cancel_clone_button: Button = builder.object("cancel_clone_button")?;
    let cancel_login_button: Button = builder.object("cancel_login_button")?;
    let login_close_button: Button = builder.object("login_close_button")?;
    let login_connect_button: Button = builder.object("login_connect_button")?;
    let login_dialog_top_label: Label = builder.object("login_dialog_top_label")?;
    let init_button: Button = builder.object("init_button")?;
    let init_dialog: Dialog = builder.object("init_dialog")?;
    let init_cancel_button: Button = builder.object("init_cancel_button")?;
    let init_accept_button: Button = builder.object("init_accept_button")?;
    let init_repo_name: Entry = builder.object("init_repo_name")?;
    let merge_branch_selector: ComboBoxText = builder.object("merge_branch_selector")?;
    let merge_button: Button = builder.object("merge_button")?;
    let add_branch_button: Button = builder.object("add_branch_button")?;
    let add_branch_dialog: Dialog = builder.object("add_branch_dialog")?;
    let branch_cancel_button: Button = builder.object("branch_cancel_button")?;
    let branch_button: Button = builder.object("branch_button")?;
    let new_branch_name: Entry = builder.object("new_branch_name")?;
    let conflict_file_chooser: FileChooserButton = builder.object("conflict_file_chooser")?;
    let conflict_text_view: TextView = builder.object("conflict_text_view")?;
    let conflict_buffer: TextBuffer = conflict_text_view.buffer()?;
    let conflict_save_button: Button = builder.object("conflict_save_button")?;
    let remote_error_label: Label = builder.object("remote_error_label")?;
    let pr_list: ListBox = builder.object("pr_list")?;
    let pr_create_button: Button = builder.object("create_pr_button")?;
    let pr_open_button: Button = builder.object("open_pr_button")?;
    let pr_closed_button: Button = builder.object("closed_pr_button")?;
    let creation_pr:Dialog = builder.object("creation_pr")?;
    let pr_cancel_button: Button = builder.object("pr_cancel_button")?;
    let pr_ok_button: Button = builder.object("pr_ok_button")?;
    let pr_title: Entry = builder.object("pr_title")?;
    let pr_descripcion: Entry = builder.object("pr_descripcion")?;
    let base_branch: ComboBoxText = builder.object("base_branch")?;
    let compare_branch: ComboBoxText = builder.object("compare_branch")?;
    
    //====Conexiones de señales====
    //====ADD BRANCH====
    let add_branch_dialog_clone = add_branch_dialog.clone();
    add_branch_button.connect_clicked(move |_| {
        add_branch_dialog_clone.show();
    });

    let add_branch_dialog_clone = add_branch_dialog.clone();
    let branch_selector_clone = branch_selector.clone();
    let merge_branch_selector_clone = merge_branch_selector.clone();
    let cliente_ = cliente.clone();
    branch_button.connect_clicked(move |_| {
        let branch_name = new_branch_name.text();
        let flags = vec![branch_name.to_string()];
        match commands_fn::branch(flags, cliente_.clone()) {
            Ok(_) => (),
            Err(_) => {
                return;
            }
        };
        update_branches(&branch_selector_clone.clone(), cliente_.clone());
        update_branches(&merge_branch_selector_clone.clone(), cliente_.clone());
        add_branch_dialog_clone.hide();
    });

    branch_cancel_button.connect_clicked(move |_| {
        add_branch_dialog.hide();
    });

    //====LOGIN====
    let connect_button_clone = connect_button.clone();
    let login_dialog_clone = login_dialog.clone();
    connect_button_clone.connect_clicked(move |_| {
        login_dialog_clone.show();
    });

    let login_dialog_clone = login_dialog.clone();
    cancel_login_button.connect_clicked(move |_| {
        login_dialog_clone.hide();
    });

    let login_button_clone = login_button.clone();
    let login_dialog_clone = login_dialog.clone();
    let cliente_clon = cliente.clone();

    login_dialog_top_label
        .set_text(format!("Hola, {}. Por favor, ingrese su mail", cliente_clon.clone()).as_str());

    login_button_clone.connect_clicked(move |_| {
        let mail = mail_entry.text().to_string();
        if !email_valido(mail.clone()) {
            login_dialog_top_label.set_text("Mail inválido. Con formato nombre@xxxxxx.yyy");
            return;
        }
        let config_file_data = format!(
            "[user]\n\temail = {}\n\tname = {}\n",
            mail,
            cliente_clon.clone()
        );
        file_manager::write_file(cliente_clon.clone() + "/gitrconfig", config_file_data).unwrap();
        login_dialog_clone.hide();
    });

    let login_dialog_clone = login_dialog.clone();
    let login_warning_clone = login_warning.clone();
    login_connect_button.connect_clicked(move |_| {
        login_warning_clone.hide();
        login_dialog_clone.show();
    });

    //====LOGIN WARNING====
    let cliente_clone = cliente.clone();
    if !existe_config(cliente_clone.clone()) {
        login_warning.show();
    }

    login_close_button.connect_clicked(move |_| {
        login_warning.hide();
    });

    //====COMMIT====
    let commit_clone = commit.clone();
    let commit_dialog_clone = commit_dialog.clone();
    commit_clone.connect_clicked(move |_| {
        commit_dialog_clone.show();
    });

    let commit_confirm_clone = commit_confirm.clone();
    let commit_dialog_clone = commit_dialog.clone();
    let cliente_ = cliente.clone();
    let remote_error_dialog_clone = remote_error_dialog.clone();
    let remote_error_label_clone = remote_error_label.clone();
    let branch_selector_clone = branch_selector.clone();

    commit_confirm_clone.connect_clicked(move |_| {
        commit_dialog_clone.hide();
        match commands_fn::add(vec![".".to_string()], cliente_.clone()) {
            Ok(_) => (),
            Err(e) => {
                if e == GitrError::FileReadError(cliente_.clone() + "/.head_repo") {
                    remote_error_label_clone
                        .set_text("No hay un repositorio asociado, busque o cree uno.");
                } else {
                    remote_error_label_clone
                        .set_text(format!("Error al hacer add: {:?}", e).as_str());
                }
                remote_error_dialog_clone.show();
            }
        };
        let message = format!("\"{}\"", commit_message.text());
        let cm_msg = vec!["-m".to_string(), message];
        let parent = match file_manager::read_file("parent".to_string()){
            Ok(parent) => parent,
            Err(_) => "None".to_string(),
        };
        match commands_fn::commit(cm_msg, parent, cliente_.clone()) {
            Ok(_) => {
                if fs::remove_file("parent").is_err(){
                    ()
                }
            },
            Err(e) => {
                println!("Error al hacer commit: {:?}", e);
                return;
            }
        };

        update_branches(&branch_selector_clone, cliente_.clone());
    });

    close_commit_dialog_button.connect_clicked(move |_| {
        commit_dialog.hide();
    });

    //====BRANCH====
    let branch_selector_clon = branch_selector.clone();
    let cliente_ = cliente.clone();

    branch_selector_clon.clone().connect_changed(move |_| {
        let branch = match branch_selector_clon.clone().active_text() {
            Some(branch) => branch,
            None => return,
        };
        let flags = vec![String::from(branch)];
        match commands_fn::checkout(flags, cliente_.clone()) {
            Ok(_) => (),
            Err(e) => {
                println!("Error al cambiar de branch: {:?}", e);
                return;
            }
        }
        let commits = get_commits(cliente_.clone());
        buffer.set_text(&commits);
    });

    let branch_selector_clon = branch_selector.clone();
    let merge_branch_selector_clon = merge_branch_selector.clone();
    let cliente_ = cliente.clone();
    let current_repo = match file_manager::get_current_repo(cliente_.clone()) {
        Ok(repo) => {
            update_branches(&branch_selector_clon, cliente_.clone());
            update_branches(&merge_branch_selector_clon, cliente_.clone());
            repo
        }
        Err(_e) => cliente_.clone(),
    };
    repo_selector.set_current_folder(current_repo.clone());
    repo_selector.connect_file_set(move |data| {
        let data_a = data.filename().unwrap();
        let repo_name = data_a.file_name().unwrap().to_str().unwrap();
        file_manager::update_current_repo(&repo_name.to_string(), cliente_.clone()).unwrap();

        update_branches(&branch_selector_clon.clone(), cliente_.clone());
        update_branches(&merge_branch_selector_clon.clone(), cliente_.clone());
    });

    //====CLONE====
    let clone_dialog_ = clone_dialog.clone();
    clone_button.connect_clicked(move |_| {
        clone_dialog_.show();
    });

    let clone_dialog_ = clone_dialog.clone();
    let cliente_ = cliente.clone();

    clone_accept_button.connect_clicked(move |_| {
        let url = clone_url.text();
        clone_dialog_.hide();
        match commands_fn::clone(
            vec![url.to_string(), "repo_clonado".to_string()],
            cliente_.clone(),
        ) {
            Ok(_) => (),
            Err(e) => {
                println!("Error al clonar: {}", e);
            }
        }
    });

    cancel_clone_button.connect_clicked(move |_| {
        clone_dialog.hide();
    });

    //====PUSH====
    let clone_push = push_button.clone();
    let clone_error = remote_error_dialog.clone();
    let cliente_ = cliente.clone();

    clone_push.connect_clicked(move |_| {
        let flags = vec![];
        match commands_fn::push(flags, cliente_.clone()) {
            Ok(_) => (),
            Err(e) => {
                println!("Error al hacer push: {:?}", e);
                clone_error.show();
                return;
            }
        };
    });
    //====PULL====
    let clone_pull = pull_button.clone();
    let clone_error = remote_error_dialog.clone();
    let cliente_ = cliente.clone();

    clone_pull.connect_clicked(move |_| {
        let flags = vec![];
        match commands_fn::pull(flags, cliente_.clone()) {
            Ok(_) => (),
            Err(e) => {
                println!("Error al hacer pull: {:?}", e);
                clone_error.show();
                return;
            }
        };
    });
    //====FETCH====
    let clone_fetch = fetch_button.clone();
    let clone_error = remote_error_dialog.clone();
    let cliente_ = cliente.clone();
    clone_fetch.connect_clicked(move |_| {
        let flags = vec![String::from("")];
        if commands_fn::fetch(flags, cliente_.clone()).is_err() {
            println!("Error al hacer fetch");
            clone_error.show();
            return;
        };
    });

    //====REMOTE ERROR DIALOG====
    let remote_error_dialog_clone = remote_error_dialog.clone();
    remote_error_close_button.connect_clicked(move |_| {
        remote_error_dialog_clone.hide();
    });

    //====INIT====
    let init_button_clone = init_button.clone();
    let init_dialog_clone = init_dialog.clone();
    init_button_clone.connect_clicked(move |_| {
        init_dialog_clone.show();
    });

    let init_dialog_clone = init_dialog.clone();
    init_cancel_button.connect_clicked(move |_| {
        init_dialog_clone.hide();
    });

    let init_dialog_clone = init_dialog.clone();
    let init_repo_name_clone = init_repo_name.clone();
    let cliente_ = cliente.clone();
    let repo_sel = repo_selector.clone();
    init_accept_button.connect_clicked(move |_| {
        let repo_name = init_repo_name_clone.text();
        init_dialog_clone.hide();
        if commands_fn::init(vec![repo_name.to_string()], cliente_.clone()).is_err() {
            println!("Error al inicializar repo");
            return;
        };
        repo_sel.set_current_folder(cliente_.clone() + "/" + repo_name.as_str());
    });

    //====MERGE====
    let merge_button_clone = merge_button.clone();
    let merge_branch_selector_clone = merge_branch_selector.clone();
    let remote_error_dialog_clone = remote_error_dialog.clone();
    let remote_error_label_clone = remote_error_label.clone();

    let cliente_ = cliente.clone();

    merge_button_clone.connect_clicked(move|_|{
        let branch = match merge_branch_selector_clone.clone().active_text(){
            Some(branch) => branch,
            None => return,
        };
        let flags = vec![branch.to_string()];
        match commands_fn::merge(flags,cliente_.clone()){
            Ok((hubo_conflict, parent, _)) => {
                if !hubo_conflict{
                    return;
                }
                remote_error_label_clone.set_text("Surgieron conflicts al hacer merge, por favor arreglarlos y commitear el resultado.");
                remote_error_dialog_clone.show();
                file_manager::write_file("parent".to_string(),parent).unwrap();           },
            Err(e) => {
                println!("Error al hacer merge: {:?}",e);
            },
        }
    });
    
    
    // ====PULL REQUESTS====
    let cliente_clone = cliente.clone();
    let clone_error = remote_error_dialog.clone();
    let base_branch_clone = base_branch.clone();
    let compare_branch_clone = compare_branch.clone();
    let pr_list_clone = pr_list.clone();
    let pr_title_clone = pr_title.clone();
    let pr_descripcion_clone = pr_descripcion.clone();
    let pr_create_button_clone = pr_create_button.clone();
    let creation_pr_clone = creation_pr.clone();
    pr_create_button_clone.connect_clicked(move |_|{
        if file_manager::get_remote(cliente_clone.clone()).is_err(){
            
            clone_error.show();
        }else{
            update_branches(&base_branch_clone, cliente_clone.clone());
            update_branches(&compare_branch_clone, cliente_clone.clone());
            pr_title_clone.set_text("");
            pr_descripcion_clone.set_text("");
            pr_title_clone.set_placeholder_text(Some("Título"));
            pr_descripcion_clone.set_placeholder_text(Some("Descripción"));
            creation_pr_clone.show();

        }
    });

    let creation_pr_clone = creation_pr.clone();
    let base_branch_clone = base_branch.clone();
    let cliente_clone = cliente.clone();
    let compare_branch_clone = compare_branch.clone();
    let pr_title_clone = pr_title.clone();
    let pr_descripcion_clone = pr_descripcion.clone();
    let pr_ok_button_clone = pr_ok_button.clone();
    pr_ok_button_clone.connect_clicked(move |_|{
        let title = pr_title_clone.clone().text();
        let description = pr_descripcion_clone.clone().text();
        let base = match base_branch_clone.active_text(){
            Some(branch) => branch,
            None => return,
        };
        let compare = match compare_branch_clone.active_text(){
            Some(branch) => branch,
            None => return,
        };
        if title == "" || description == "" || base == "" || compare == ""{
            return;
        }
        let vec_pr = vec![title.to_string(),description.to_string(),compare.to_string(),base.to_string()];
        _create_pr(vec_pr,cliente_clone.clone()).unwrap();
        //crear el pr
        creation_pr_clone.hide();
    });
    let pr_cancel_button_clone = pr_cancel_button.clone();
    let creation_pr_clone = creation_pr.clone();
    pr_cancel_button_clone.connect_clicked(move |_|{
        creation_pr_clone.hide();
    });
    let cliente_clone = cliente.clone();
    let clone_error = remote_error_dialog.clone();
    let pr_open_button = pr_open_button.clone();
    pr_open_button.connect_clicked(move |_|{
        if file_manager::get_remote(cliente_clone.clone()).is_err(){
            clone_error.show();
        }else{
            pr_list_clone.foreach(|row|{
                pr_list_clone.remove(row);
            });
            let remote = file_manager::get_remote(cliente_clone.clone()).unwrap();
            let sv_url = remote.split('/').collect::<Vec<&str>>()[0].replace("localhost:", "server");
            let sv_name = remote.split('/').collect::<Vec<&str>>()[1];
            let dir = sv_url + "/repos/" + sv_name;
            let prs = match file_manager::get_pull_requests(dir){
                Ok(prs) => prs,
                Err(e) => {
                    println!("Error al obtener pull requests: {:?}",e);
                    clone_error.show();
                    return;
                }
            };
            for pr in prs.iter().filter(|pr| pr.status == "open"){
                let row = create_pull_request_row(pr);
                pr_list_clone.add(&row);
            }
            pr_list_clone.show_all();  
        }
    });
    let cliente_clone = cliente.clone();
    let clone_error = remote_error_dialog.clone();
    let pr_closed_button = pr_closed_button.clone();
    let pr_list_clone = pr_list.clone();
    pr_closed_button.connect_clicked(move |_|{
        if file_manager::get_remote(cliente_clone.clone()).is_err(){
            clone_error.show();
        }else{
            pr_list_clone.foreach(|row|{
                pr_list_clone.remove(row);
            });
            let remote = file_manager::get_remote(cliente_clone.clone()).unwrap();
            let sv_url = remote.split('/').collect::<Vec<&str>>()[0].replace("localhost:", "server");
            let sv_name = remote.split('/').collect::<Vec<&str>>()[1];
            let dir = sv_url + "/repos/" + sv_name;
            let prs = match file_manager::get_pull_requests(dir){
                Ok(prs) => prs,
                Err(e) => {
                    println!("Error al obtener pull requests: {:?}",e);
                    clone_error.show();
                    return;
                }
            };
            for pr in prs.iter().filter(|pr| pr.status == "closed"){
                let row = create_pull_request_row(pr);
                pr_list_clone.add(&row);
            }
            pr_list_clone.show_all();  
        }
    });



    //====CONFLICTS====
    let conflict_file_chooser_clone = conflict_file_chooser.clone();
    let cliente_ = cliente.clone();
    let conf_buffer = conflict_buffer.clone();
    conflict_file_chooser_clone.set_current_folder(cliente_.clone());
    conflict_file_chooser.connect_file_set(move |data| {
        let filename = data.filename().unwrap().to_str().unwrap().to_string();
        let data_from_file = file_manager::read_file(filename.clone()).unwrap();
        conf_buffer.set_text(&data_from_file);
    });

    let conf_buffer = conflict_buffer.clone();
    conflict_save_button.connect_clicked(move |_| {
        let filename = conflict_file_chooser_clone
            .filename()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let data = conf_buffer
            .text(&conf_buffer.start_iter(), &conf_buffer.end_iter(), false)
            .unwrap()
            .to_string();
        file_manager::write_file(filename.clone(), data).unwrap();
    });

    window.set_application(Some(application));
    window.show_all();
    Some("Ok".to_string())
}

pub fn initialize_gui(cliente: String) {
    let app = Application::new(Some("test.test"), ApplicationFlags::HANDLES_OPEN);
    let cliente_clon = cliente.clone();
    app.connect_open(move |app, _files, _| {
        build_ui(app, cliente_clon.clone());
    });

    app.run();
}


fn create_pull_request_row(pull_request: &PullRequest) -> ListBoxRow {
    let id_label = Label::new(Some(&format!("ID: {}", pull_request.id)));
    let title_label = Label::new(Some(&format!("Título: {}", pull_request.title)));
    let description_label = Label::new(Some(&format!("Descripción: {}", pull_request.description)));
    let branches = Label::new(Some(&format!("{}==>{}", pull_request.head, pull_request.base)));

    let row_box = gtk::Box::new(Orientation::Vertical, 5);
    row_box.pack_start(&id_label, false, false, 0);
    row_box.pack_start(&title_label, false, false, 0);
    row_box.pack_start(&description_label, false, false, 0);
    row_box.pack_start(&branches, false, false, 0);


    let row = ListBoxRow::new();
    row.add(&row_box);
    row
}