-- https://ignite.apache.org/docs/latest/sql-reference/data-types
create table rainbow (
--      id UUID, -- java.util.UUID -- TODO: add support for this type
     big BIGINT, -- java.lang.Long
     bool BOOLEAN, -- java.lang.Boolean
     dec DECIMAL, -- java.math.BigDecimal
--      double DOUBLE, -- java.lang.Double -- TODO: add support for this type
     int INT, -- java.lang.Integer
     null_int INT, -- java.lang.Integer
--      real REAL, -- java.lang.Float -- TODO: add support for this type
     small SMALLINT, -- java.lang.Short
     tiny TINYINT, -- java.lang.Byte
     char CHAR, -- java.lang.String
     var VARCHAR, -- java.lang.String
--      date DATE, -- java.sql.Date -- TODO: add support for this type
--      time TIME, -- java.sql.Time -- TODO: add support for this type
     ts TIMESTAMP, -- java.sql.Timestamp
     bin BINARY, -- byte[] 
     primary key (big)
);

insert into rainbow (big, bool, dec, int, null_int, small, tiny, char, var, ts, bin) values
    (1, true, 2.0, 3, null, 4, 5, 'c', 'varchar', timestamp '2023-06-21 12:34:56 UTC', x'1234567890abcdef');
