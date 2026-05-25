-- Add migration script here

# The table of User
# Note : gender is either 'F'(Female) or 'M'(Male)
CREATE TABLE IF NOT EXISTS Users (
    id         INT PRIMARY KEY AUTO_INCREMENT,
    password   VARCHAR(50) NOT NULL,
    name       VARCHAR(20),
    gender     CHAR(1) CHECK (gender IN ('M', 'F')),
    birth_date DATE,
    age        INT   
);

CREATE TABLE IF NOT EXISTS Relations (
    user_id    BIGINT UNSIGNED NOT NULL,
    friend_id  BIGINT UNSIGNED NOT NULL,
    -- 0: 待处理的申请 (单向)
    -- 2: 正式好友 (双向互为记录)
    status     TINYINT UNSIGNED NOT NULL DEFAULT 0, 
    group_id   BIGINT UNSIGNED NULL,
    PRIMARY KEY (user_id, friend_id),
    -- 高频查询索引：用于“谁加了我”和“搜索我的好友”
    INDEX idx_friend_status (friend_id, status),
    INDEX idx_user_status (user_id, status),
    CONSTRAINT fk_rel_user FOREIGN KEY (user_id) REFERENCES Users(id) ON DELETE CASCADE,
    CONSTRAINT fk_rel_friend FOREIGN KEY (friend_id) REFERENCES Users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS FriendGroups (
    group_id   BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
    user_id    BIGINT UNSIGNED NOT NULL,
    name       VARCHAR(32) NOT NULL,

    UNIQUE KEY uk_user_group (user_id, name),
    CONSTRAINT fk_group_user FOREIGN KEY (user_id) REFERENCES Users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Moments (
    author_id BIGINT UNSIGNED NOT NULL,
    content VARCHAR(150),
    last_modified_time TIMESTAMP,

    UNIQUE KEY uk_id_content (author_id, content),
    CONSTRAINT fk_author FOREIGN KEY (author_id) REFERENCES Users(id) ON DELETE CASCADE
);