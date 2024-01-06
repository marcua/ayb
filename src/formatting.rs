use prettytable::{format, Table};

pub trait TabularFormatter {
    fn to_table(&self) -> Table;
    fn generate_table(&self) -> Result<(), std::io::Error> {
        let mut table = self.to_table();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.print(&mut std::io::stdout())?;
        Ok(())
    }
    fn generate_csv(&self) -> Result<(), std::io::Error> {
        let table = self.to_table();
        table.to_csv(std::io::stdout())?;
        Ok(())
    }
}
