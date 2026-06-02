-- Add migration script here

# The table of User
# Note : gender is either 'F'(Female) or 'M'(Male)
# id is set as BIGINT UNSIGNED in accordance with u64
# "Age" is not needed here because it should be calculated
CREATE TABLE IF NOT EXISTS Users (
    id         BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
    password   VARCHAR(50) NOT NULL,
    name       VARCHAR(20),
    gender     CHAR(1) CHECK (gender IN ('M', 'F')),
    birth_date DATE,
    # 0 代表普通用户，1 代表系统管理员
    role       TINYINT UNSIGNED NOT NULL DEFAULT 0
);

INSERT INTO Users (password, name, role) 
VALUES ('adminpass', 'System_Root', 1);

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

CREATE TABLE Moments (
    moment_id BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
    author_id BIGINT UNSIGNED NOT NULL,
    content VARCHAR(150) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_modified_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP NOT NULL,

    INDEX idx_author_time (author_id, created_at DESC),
    CONSTRAINT fk_moment_author FOREIGN KEY (author_id) REFERENCES Users(id) ON DELETE CASCADE
);

CREATE TABLE Comments (
    comment_id BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
    moment_id BIGINT UNSIGNED NOT NULL,
    commenter_id BIGINT UNSIGNED NOT NULL,
    comment VARCHAR(50) NOT NULL,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- 优化查询防线：查询某条朋友圈下的所有评论，通常按时间正序排列
    INDEX idx_moment_time (moment_id, created_at ASC),

    CONSTRAINT fk_comment_moment FOREIGN KEY (moment_id) REFERENCES Moments(moment_id) ON DELETE CASCADE,
    CONSTRAINT fk_comment_user FOREIGN KEY (commenter_id) REFERENCES Users(id) ON DELETE CASCADE
);
