use super::*;

#[test]
fn images_with_use() {
    let text = "@@IBC:MAIN@@\n\
        0123456789ab\tnginx\t1.27\t52.3MB\t3 weeks ago\n\
        aaaaaaaaaaaa\tpostgres\tlatest\t1.2GB\t2 months ago\n\
        bbbbbbbbbbbb\t<none>\t<none>\t5.57kB\t1 year ago\n\
        @@IBC:USE@@\nnginx:1.27\npostgres\n@@IBC:END@@\n";
    let r = parse_images(text);
    assert_eq!(r.access, DockerAccess::Ok);
    assert_eq!(r.images.len(), 3);
    // Used first, then biggest first.
    assert_eq!(r.images[0].name, "postgres:latest");
    assert_eq!(r.images[0].containers, 1);
    assert_eq!(r.images[1].name, "nginx:1.27");
    assert_eq!(r.images[2].name, "<none>");
    assert_eq!(r.images[2].containers, 0);
    assert_eq!(r.images[2].size_bytes, Some(5570));
}

#[test]
fn volumes_sizes_and_mounts() {
    let text = "@@IBC:MAIN@@\npgdata\tlocal\nunused\tlocal\n\
        @@IBC:USE@@\npgdata,/etc/app.conf\n/srv/www\n\
        @@IBC:DF@@\n\nVOLUME NAME   LINKS   SIZE\npgdata   1   412.5MB\nunused   0   0B\n\n\
        @@IBC:END@@\n";
    let r = parse_volumes(text);
    assert_eq!(r.volumes[0].name, "pgdata");
    assert_eq!(r.volumes[0].containers, 1);
    assert_eq!(r.volumes[0].size_bytes, Some(412_500_000));
    assert_eq!(r.volumes[1].containers, 0);
    assert_eq!(r.volumes[1].size_bytes, Some(0));
}

#[test]
fn networks_with_subnets() {
    let text = "@@IBC:MAIN@@\n\
        aaaaaaaaaaaa\tbridge\tbridge\tlocal\n\
        bbbbbbbbbbbb\tapp_default\tbridge\tlocal\n\
        @@IBC:INSPECT@@\n\
        aaaaaaaaaaaa1111\t172.17.0.0/16 \t0\n\
        bbbbbbbbbbbb2222\t172.20.0.0/16 \t3\n@@IBC:END@@\n";
    let r = parse_networks(text);
    assert_eq!(r.networks[0].name, "app_default");
    assert_eq!(r.networks[0].containers, 3);
    assert_eq!(r.networks[0].subnet, "172.20.0.0/16");
    assert_eq!(r.networks[1].containers, 0);
}

#[test]
fn compose_groups_by_label() {
    let text = "@@IBC:MAIN@@\n\
        a1b2c3d4e5f6\tshop-web-1\trunning\tUp 2 days\tshop\tweb\t/srv/shop\n\
        b1b2c3d4e5f6\tshop-db-1\texited\tExited (1) 1 hour ago\tshop\tdb\t/srv/shop\n\
        c1b2c3d4e5f6\tstandalone\trunning\tUp 1 day\t\t\t\n@@IBC:END@@\n";
    let r = parse_compose(text);
    assert_eq!(r.projects.len(), 1);
    let p = &r.projects[0];
    assert_eq!((p.name.as_str(), p.running, p.total), ("shop", 1, 2));
    assert_eq!(p.dir.as_deref(), Some("/srv/shop"));
    assert_eq!(p.services[0].service, "db");
}

#[test]
fn events_newest_first_without_noise() {
    let text = "@@IBC:MAIN@@\n\
        1700000000\tcontainer\tstart\tweb\n\
        1700000005\tcontainer\texec_start: sh -c secret\tweb\n\
        1700000006\tcontainer\thealth_status: healthy\tweb\n\
        1700000010\tcontainer\tdie\tweb\n\
        garbage\n@@IBC:END@@\n";
    let r = parse_events(text);
    assert_eq!(r.events.len(), 2);
    assert_eq!(r.events[0].action, "die");
    assert_eq!(r.events[0].time, 1_700_000_010_000);
    assert_eq!(r.events[1].name, "web");
}

#[test]
fn access_states() {
    assert_eq!(
        parse_images("@@IBC:MISSING@@\n").access,
        DockerAccess::Missing
    );
    let denied = "@@IBC:MAIN@@\nGot permission denied while trying to connect\n@@IBC:END@@\n";
    assert_eq!(parse_events(denied).access, DockerAccess::Denied);
    let down = "@@IBC:MAIN@@\nCannot connect to the Docker daemon. Is the docker daemon running?\n";
    assert_eq!(parse_compose(down).access, DockerAccess::Down);
}

#[test]
fn script_uses_tabs_and_ends() {
    let s = script(IMAGES);
    assert!(s.contains("{{.ID}}\t{{.Repository}}"));
    assert!(!s.contains("<T>"));
    assert!(s.ends_with("@@IBC:END@@'\n"));
}
