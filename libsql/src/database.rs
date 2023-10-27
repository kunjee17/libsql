use std::fmt;

use crate::{Connection, Result};

cfg_core! {
    bitflags::bitflags! {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[repr(C)]
        pub struct OpenFlags: ::std::os::raw::c_int {
            const SQLITE_OPEN_READ_ONLY = libsql_sys::ffi::SQLITE_OPEN_READONLY as i32;
            const SQLITE_OPEN_READ_WRITE = libsql_sys::ffi::SQLITE_OPEN_READWRITE as i32;
            const SQLITE_OPEN_CREATE = libsql_sys::ffi::SQLITE_OPEN_CREATE as i32;
        }
    }

    impl Default for OpenFlags {
        #[inline]
        fn default() -> OpenFlags {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        }
    }
}

// TODO(lucio): Improve construction via
//      1) Move open errors into open fn rather than connect
//      2) Support replication setup
enum DbType {
    #[cfg(feature = "core")]
    Memory,
    #[cfg(feature = "core")]
    File { path: String, flags: OpenFlags },
    #[cfg(feature = "replication")]
    Sync { db: crate::local::Database },
    #[cfg(feature = "hrana")]
    Remote {
        url: String,
        auth_token: String,
        connector: crate::util::ConnectorService,
    },
}

impl fmt::Debug for DbType {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "core")]
            Self::Memory => write!(f, "Memory"),
            #[cfg(feature = "core")]
            Self::File { .. } => write!(f, "File"),
            #[cfg(feature = "replication")]
            Self::Sync { .. } => write!(f, "Sync"),
            #[cfg(feature = "hrana")]
            Self::Remote { .. } => write!(f, "Remote"),
            _ => write!(f, "no database type set"),
        }
    }
}

pub struct Database {
    db_type: DbType,
}

cfg_core! {
    impl Database {
        pub fn open_in_memory() -> Result<Self> {
            Ok(Database {
                db_type: DbType::Memory,
            })
        }

        pub fn open(db_path: impl Into<String>) -> Result<Database> {
            Database::open_with_flags(db_path, OpenFlags::default())
        }

        pub fn open_with_flags(db_path: impl Into<String>, flags: OpenFlags) -> Result<Database> {
            Ok(Database {
                db_type: DbType::File {
                    path: db_path.into(),
                    flags,
                },
            })
        }
    }
}

cfg_replication! {
    use crate::Error;

    impl Database {
        /// Open a local database file with the ability to sync from snapshots from local filesystem.
        #[cfg(feature = "replication")]
        pub async fn open_with_local_sync(db_path: impl Into<String>) -> Result<Database> {
            Ok(Database {
                db_type: DbType::File { path: db_path.into(), flags: OpenFlags::default() },
            })
        }

        /// Open a local database file with the ability to sync from a remote database.
        #[cfg(feature = "replication")]
        pub async fn open_with_remote_sync(
            db_path: impl Into<String>,
            url: impl Into<String>,
            token: impl Into<String>,
        ) -> Result<Database> {
            let mut http = hyper::client::HttpConnector::new();
            http.enforce_http(false);
            http.set_nodelay(true);

            Self::open_with_remote_sync_connector(db_path, url, token, http)
        }

        #[doc(hidden)]
        pub fn open_with_remote_sync_connector<C>(
            db_path: impl Into<String>,
            url: impl Into<String>,
            token: impl Into<String>,
            connector: C,
        ) -> Result<Database>
        where
            C: tower::Service<http::Uri> + Send + Clone + Sync + 'static,
            C::Response: crate::util::Socket,
            C::Future: Send + 'static,
            C::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            use tower::ServiceExt;

            let svc = connector
                .map_err(|e| e.into())
                .map_response(|s| Box::new(s) as Box<dyn crate::util::Socket>);

            let svc = crate::util::ConnectorService::new(svc);

            let db = crate::local::Database::open_http_sync(
                svc,
                db_path.into(),
                url.into(),
                token.into()
            )?;
            Ok(Database {
                db_type: DbType::Sync { db },
            })
        }


        pub async fn sync(&self) -> Result<usize> {
            if let DbType::Sync { db } = &self.db_type {
                db.sync().await
            } else {
                Err(Error::SyncNotSupported(format!("{:?}", self.db_type)))
            }
        }

        pub fn sync_frames(&self, frames: crate::replication::Frames) -> Result<usize> {
            if let DbType::Sync { db } = &self.db_type {
                db.sync_frames(frames)
            } else {
                Err(Error::SyncNotSupported(format!("{:?}", self.db_type)))
            }
        }
    }
}

cfg_hrana! {
    impl Database {
        pub fn open_remote(url: impl Into<String>, auth_token: impl Into<String>) -> Result<Self> {
            let mut connector = hyper::client::HttpConnector::new();
            connector.enforce_http(false);

            Self::open_remote_with_connector(url, auth_token, connector)
        }

        // For now, only expose this for sqld testing purposes
        #[doc(hidden)]
        pub fn open_remote_with_connector<C>(
            url: impl Into<String>,
            auth_token: impl Into<String>,
            connector: C,
        ) -> Result<Self>
        where
            C: tower::Service<http::Uri> + Send + Clone + Sync + 'static,
            C::Response: crate::util::Socket,
            C::Future: Send + 'static,
            C::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            use tower::ServiceExt;

            let svc = connector
                .map_err(|e| e.into())
                .map_response(|s| Box::new(s) as Box<dyn crate::util::Socket>);
            Ok(Database {
                db_type: DbType::Remote {
                    url: url.into(),
                    auth_token: auth_token.into(),
                    connector: crate::util::ConnectorService::new(svc),
                },
            })
        }
    }
}

impl Database {
    #[allow(unreachable_patterns)]
    pub fn connect(&self) -> Result<Connection> {
        match &self.db_type {
            #[cfg(feature = "core")]
            DbType::Memory => {
                use crate::local::impls::LibsqlConnection;

                let db = crate::local::Database::open(":memory:", OpenFlags::default())?;
                let conn = db.connect()?;

                let conn = std::sync::Arc::new(LibsqlConnection { conn });

                Ok(Connection { conn })
            }

            #[cfg(feature = "core")]
            DbType::File { path, flags } => {
                use crate::local::impls::LibsqlConnection;

                let db = crate::local::Database::open(path, *flags)?;
                let conn = db.connect()?;

                let conn = std::sync::Arc::new(LibsqlConnection { conn });

                Ok(Connection { conn })
            }

            #[cfg(feature = "replication")]
            DbType::Sync { db } => {
                use crate::local::impls::LibsqlConnection;

                let conn = db.connect()?;

                let local = LibsqlConnection { conn };
                let writer = local.conn.writer().unwrap().clone();

                let remote = crate::replication::RemoteConnection::new(local, writer);

                let conn = std::sync::Arc::new(remote);

                Ok(Connection { conn })
            }

            #[cfg(feature = "hrana")]
            DbType::Remote {
                url,
                auth_token,
                connector,
            } => {
                let conn = std::sync::Arc::new(crate::hrana::Client::new_with_connector(
                    url,
                    auth_token,
                    connector.clone(),
                ));

                Ok(Connection { conn })
            }

            _ => unreachable!("no database type set"),
        }
    }
}