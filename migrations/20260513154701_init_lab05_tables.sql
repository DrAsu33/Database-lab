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