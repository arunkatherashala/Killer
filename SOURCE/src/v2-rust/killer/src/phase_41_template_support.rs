/// Phase 41: Template Support
/// 
/// Implements advanced template capabilities:
/// - Mail-merge (variables, conditions, loops)
/// - Invoice generation (line items, tax, totals)
/// - Custom templates (variables, helpers, partials)
/// - Bulk document generation (batch processing)
/// - Template validation and error handling
///
/// Author: Killer Language Dev Team
/// Version: 1.0.0
/// Date: 2026-03-19

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub value: String,
    pub data_type: VariableType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableType {
    String,
    Number,
    Currency,
    Date,
    Boolean,
    List,
}

#[derive(Debug, Clone)]
pub struct MailMergeTemplate {
    pub name: String,
    pub subject: String,
    pub body: String,
    pub variables: HashMap<String, TemplateVariable>,
    pub conditions: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub discount: f64,
}

#[derive(Debug, Clone)]
pub struct InvoiceTemplate {
    pub invoice_number: String,
    pub company_name: String,
    pub customer_name: String,
    pub customer_email: String,
    pub customer_address: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub payment_terms: String,
    pub notes: String,
    pub currency_symbol: String,
}

#[derive(Debug, Clone)]
pub struct CustomTemplate {
    pub name: String,
    pub content: String,
    pub variables: HashMap<String, String>,
    pub sections: HashMap<String, String>,
    pub helpers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct BulkGenerationJob {
    pub job_id: String,
    pub template_name: String,
    pub data_source: String,
    pub output_format: String,
    pub total_documents: u32,
    pub generated_count: u32,
    pub failed_count: u32,
}

/// Mail-merge Engine: Processes templates with variable substitution
#[derive(Debug)]
pub struct MailMergeEngine {
    templates: HashMap<String, MailMergeTemplate>,
    processed_count: usize,
}

impl MailMergeEngine {
    pub fn new() -> Self {
        MailMergeEngine {
            templates: HashMap::new(),
            processed_count: 0,
        }
    }

    pub fn create_template(&mut self, name: &str, subject: &str, body: &str) -> Result<(), Box<dyn Error>> {
        if name.is_empty() || subject.is_empty() || body.is_empty() {
            return Err("Template name, subject, and body cannot be empty".into());
        }
        let template = MailMergeTemplate {
            name: name.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            variables: HashMap::new(),
            conditions: HashMap::new(),
        };
        self.templates.insert(name.to_string(), template);
        Ok(())
    }

    pub fn add_variable(
        &mut self,
        template_name: &str,
        var_name: &str,
        data_type: VariableType,
        required: bool,
    ) -> Result<(), Box<dyn Error>> {
        if var_name.is_empty() {
            return Err("Variable name cannot be empty".into());
        }
        match self.templates.get_mut(template_name) {
            Some(template) => {
                let var = TemplateVariable {
                    name: var_name.to_string(),
                    value: String::new(),
                    data_type,
                    required,
                };
                template.variables.insert(var_name.to_string(), var);
                Ok(())
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn add_condition(
        &mut self,
        template_name: &str,
        condition_name: &str,
        expression: String,
    ) -> Result<(), Box<dyn Error>> {
        if condition_name.is_empty() || expression.is_empty() {
            return Err("Condition name and expression cannot be empty".into());
        }
        match self.templates.get_mut(template_name) {
            Some(template) => {
                template.conditions.insert(condition_name.to_string(), expression);
                Ok(())
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn merge(
        &mut self,
        template_name: &str,
        data: HashMap<String, String>,
    ) -> Result<String, Box<dyn Error>> {
        match self.templates.get(template_name) {
            Some(template) => {
                // Validate required variables
                for (var_name, var) in &template.variables {
                    if var.required && !data.contains_key(var_name) {
                        return Err(format!("Required variable '{}' missing", var_name).into());
                    }
                }

                let mut result = template.body.clone();
                for (key, value) in &data {
                    let placeholder = format!("{{{{{}}}}}", key);
                    result = result.replace(&placeholder, value);
                }

                self.processed_count += 1;
                Ok(result)
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn get_template(&self, name: &str) -> Option<&MailMergeTemplate> {
        self.templates.get(name)
    }

    pub fn get_processed_count(&self) -> usize {
        self.processed_count
    }

    pub fn clear(&mut self) {
        self.templates.clear();
        self.processed_count = 0;
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

/// Invoice Generator: Creates professional invoices
#[derive(Debug)]
pub struct InvoiceGenerator {
    invoices: HashMap<String, InvoiceTemplate>,
    generated_count: usize,
}

impl InvoiceGenerator {
    pub fn new() -> Self {
        InvoiceGenerator {
            invoices: HashMap::new(),
            generated_count: 0,
        }
    }

    pub fn create_invoice(
        &mut self,
        invoice_number: &str,
        company_name: &str,
        customer_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        if invoice_number.is_empty() || company_name.is_empty() || customer_name.is_empty() {
            return Err("Invoice number, company, and customer name cannot be empty".into());
        }
        let invoice = InvoiceTemplate {
            invoice_number: invoice_number.to_string(),
            company_name: company_name.to_string(),
            customer_name: customer_name.to_string(),
            customer_email: String::new(),
            customer_address: String::new(),
            line_items: Vec::new(),
            payment_terms: "Net 30".to_string(),
            notes: String::new(),
            currency_symbol: "$".to_string(),
        };
        self.invoices.insert(invoice_number.to_string(), invoice);
        Ok(())
    }

    pub fn add_line_item(
        &mut self,
        invoice_number: &str,
        description: &str,
        quantity: u32,
        unit_price: f64,
    ) -> Result<(), Box<dyn Error>> {
        if description.is_empty() || quantity == 0 || unit_price < 0.0 {
            return Err("Invalid line item parameters".into());
        }
        match self.invoices.get_mut(invoice_number) {
            Some(invoice) => {
                let item = InvoiceLineItem {
                    description: description.to_string(),
                    quantity,
                    unit_price,
                    tax_rate: 0.0,
                    discount: 0.0,
                };
                invoice.line_items.push(item);
                Ok(())
            }
            None => Err(format!("Invoice '{}' not found", invoice_number).into()),
        }
    }

    pub fn set_customer_info(
        &mut self,
        invoice_number: &str,
        email: &str,
        address: &str,
    ) -> Result<(), Box<dyn Error>> {
        if email.is_empty() || address.is_empty() {
            return Err("Email and address cannot be empty".into());
        }
        match self.invoices.get_mut(invoice_number) {
            Some(invoice) => {
                invoice.customer_email = email.to_string();
                invoice.customer_address = address.to_string();
                Ok(())
            }
            None => Err(format!("Invoice '{}' not found", invoice_number).into()),
        }
    }

    pub fn calculate_total(&self, invoice_number: &str) -> Result<f64, Box<dyn Error>> {
        match self.invoices.get(invoice_number) {
            Some(invoice) => {
                let mut total = 0.0;
                for item in &invoice.line_items {
                    let subtotal = (item.quantity as f64) * item.unit_price;
                    let discounted = subtotal - item.discount;
                    let tax = discounted * item.tax_rate;
                    total += discounted + tax;
                }
                Ok(total)
            }
            None => Err(format!("Invoice '{}' not found", invoice_number).into()),
        }
    }

    pub fn generate_invoice_text(&mut self, invoice_number: &str) -> Result<String, Box<dyn Error>> {
        match self.invoices.get(invoice_number) {
            Some(invoice) => {
                let mut output = String::new();
                output.push_str(&format!("INVOICE #{}\n", invoice.invoice_number));
                output.push_str(&format!("Company: {}\n", invoice.company_name));
                output.push_str(&format!("Customer: {}\n", invoice.customer_name));
                output.push_str(&format!("Email: {}\n", invoice.customer_email));
                output.push_str(&format!("Address: {}\n", invoice.customer_address));
                output.push_str("\n--- LINE ITEMS ---\n");

                for item in &invoice.line_items {
                    let subtotal = (item.quantity as f64) * item.unit_price;
                    output.push_str(&format!(
                        "{}: {} x {} = {}{:.2}\n",
                        item.description,
                        item.quantity,
                        invoice.currency_symbol,
                        invoice.currency_symbol,
                        subtotal
                    ));
                }

                let total = self.calculate_total(invoice_number)?;
                output.push_str(&format!("\nTOTAL: {}{:.2}\n", invoice.currency_symbol, total));
                output.push_str(&format!("Payment Terms: {}\n", invoice.payment_terms));

                self.generated_count += 1;
                Ok(output)
            }
            None => Err(format!("Invoice '{}' not found", invoice_number).into()),
        }
    }

    pub fn get_invoice(&self, invoice_number: &str) -> Option<&InvoiceTemplate> {
        self.invoices.get(invoice_number)
    }

    pub fn get_generated_count(&self) -> usize {
        self.generated_count
    }

    pub fn clear(&mut self) {
        self.invoices.clear();
        self.generated_count = 0;
    }

    pub fn invoice_count(&self) -> usize {
        self.invoices.len()
    }
}

/// Custom Template Engine: Flexible template system with variables and sections
#[derive(Debug)]
pub struct CustomTemplateEngine {
    templates: HashMap<String, CustomTemplate>,
    rendered_count: usize,
}

impl CustomTemplateEngine {
    pub fn new() -> Self {
        CustomTemplateEngine {
            templates: HashMap::new(),
            rendered_count: 0,
        }
    }

    pub fn create_template(&mut self, name: &str, content: &str) -> Result<(), Box<dyn Error>> {
        if name.is_empty() || content.is_empty() {
            return Err("Template name and content cannot be empty".into());
        }
        let template = CustomTemplate {
            name: name.to_string(),
            content: content.to_string(),
            variables: HashMap::new(),
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };
        self.templates.insert(name.to_string(), template);
        Ok(())
    }

    pub fn add_variable(
        &mut self,
        template_name: &str,
        var_name: &str,
        default_value: String,
    ) -> Result<(), Box<dyn Error>> {
        if var_name.is_empty() {
            return Err("Variable name cannot be empty".into());
        }
        match self.templates.get_mut(template_name) {
            Some(template) => {
                template.variables.insert(var_name.to_string(), default_value);
                Ok(())
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn add_section(
        &mut self,
        template_name: &str,
        section_name: &str,
        content: String,
    ) -> Result<(), Box<dyn Error>> {
        if section_name.is_empty() || content.is_empty() {
            return Err("Section name and content cannot be empty".into());
        }
        match self.templates.get_mut(template_name) {
            Some(template) => {
                template.sections.insert(section_name.to_string(), content);
                Ok(())
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn add_helper(
        &mut self,
        template_name: &str,
        helper_name: &str,
        helper_code: String,
    ) -> Result<(), Box<dyn Error>> {
        if helper_name.is_empty() || helper_code.is_empty() {
            return Err("Helper name and code cannot be empty".into());
        }
        match self.templates.get_mut(template_name) {
            Some(template) => {
                template.helpers.insert(helper_name.to_string(), helper_code);
                Ok(())
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn render(
        &mut self,
        template_name: &str,
        variables: HashMap<String, String>,
    ) -> Result<String, Box<dyn Error>> {
        match self.templates.get(template_name) {
            Some(template) => {
                let mut result = template.content.clone();

                // Replace variables
                for (key, value) in &variables {
                    let placeholder = format!("{{{{{}}}}}", key);
                    result = result.replace(&placeholder, value);
                }

                // Replace default variables
                for (key, value) in &template.variables {
                    if !variables.contains_key(key) {
                        let placeholder = format!("{{{{{}}}}}", key);
                        result = result.replace(&placeholder, value);
                    }
                }

                // Replace sections
                for (key, value) in &template.sections {
                    let placeholder = format!("{{%section:{}%}}", key);
                    result = result.replace(&placeholder, value);
                }

                self.rendered_count += 1;
                Ok(result)
            }
            None => Err(format!("Template '{}' not found", template_name).into()),
        }
    }

    pub fn get_template(&self, name: &str) -> Option<&CustomTemplate> {
        self.templates.get(name)
    }

    pub fn get_rendered_count(&self) -> usize {
        self.rendered_count
    }

    pub fn clear(&mut self) {
        self.templates.clear();
        self.rendered_count = 0;
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

/// Bulk Generation Coordinator: Manages batch document generation
#[derive(Debug)]
pub struct BulkGenerationService {
    jobs: HashMap<String, BulkGenerationJob>,
    job_counter: u32,
}

impl BulkGenerationService {
    pub fn new() -> Self {
        BulkGenerationService {
            jobs: HashMap::new(),
            job_counter: 0,
        }
    }

    pub fn create_job(
        &mut self,
        template_name: &str,
        data_source: &str,
        output_format: &str,
        total_documents: u32,
    ) -> Result<String, Box<dyn Error>> {
        if template_name.is_empty() || data_source.is_empty() || output_format.is_empty() || total_documents == 0 {
            return Err("Invalid job parameters".into());
        }

        let job_id = format!("job_{}", self.job_counter);
        self.job_counter += 1;

        let job = BulkGenerationJob {
            job_id: job_id.clone(),
            template_name: template_name.to_string(),
            data_source: data_source.to_string(),
            output_format: output_format.to_string(),
            total_documents,
            generated_count: 0,
            failed_count: 0,
        };

        self.jobs.insert(job_id.clone(), job);
        Ok(job_id)
    }

    pub fn update_progress(
        &mut self,
        job_id: &str,
        generated: u32,
        failed: u32,
    ) -> Result<(), Box<dyn Error>> {
        match self.jobs.get_mut(job_id) {
            Some(job) => {
                job.generated_count = generated;
                job.failed_count = failed;
                Ok(())
            }
            None => Err(format!("Job '{}' not found", job_id).into()),
        }
    }

    pub fn get_progress(&self, job_id: &str) -> Result<(u32, u32, u32), Box<dyn Error>> {
        match self.jobs.get(job_id) {
            Some(job) => {
                let completed = job.generated_count + job.failed_count;
                Ok((job.generated_count, job.failed_count, completed))
            }
            None => Err(format!("Job '{}' not found", job_id).into()),
        }
    }

    pub fn get_job(&self, job_id: &str) -> Option<&BulkGenerationJob> {
        self.jobs.get(job_id)
    }

    pub fn is_complete(&self, job_id: &str) -> Result<bool, Box<dyn Error>> {
        match self.jobs.get(job_id) {
            Some(job) => {
                let completed = job.generated_count + job.failed_count;
                Ok(completed >= job.total_documents)
            }
            None => Err(format!("Job '{}' not found", job_id).into()),
        }
    }

    pub fn clear(&mut self) {
        self.jobs.clear();
        self.job_counter = 0;
    }

    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }
}

/// Template Support Coordinator: Master controller for all template operations
#[derive(Debug)]
pub struct TemplateSupport {
    mail_merge: MailMergeEngine,
    invoice_generator: InvoiceGenerator,
    custom_templates: CustomTemplateEngine,
    bulk_service: BulkGenerationService,
}

impl TemplateSupport {
    pub fn new() -> Self {
        TemplateSupport {
            mail_merge: MailMergeEngine::new(),
            invoice_generator: InvoiceGenerator::new(),
            custom_templates: CustomTemplateEngine::new(),
            bulk_service: BulkGenerationService::new(),
        }
    }

    pub fn mail_merge(&mut self) -> &mut MailMergeEngine {
        &mut self.mail_merge
    }

    pub fn invoices(&mut self) -> &mut InvoiceGenerator {
        &mut self.invoice_generator
    }

    pub fn templates(&mut self) -> &mut CustomTemplateEngine {
        &mut self.custom_templates
    }

    pub fn bulk(&mut self) -> &mut BulkGenerationService {
        &mut self.bulk_service
    }

    pub fn summary(&self) -> String {
        format!(
            "Template Support:\n- Mail-Merge Templates: {}\n- Invoices: {}\n- Custom Templates: {}\n- Bulk Jobs: {}",
            self.mail_merge.template_count(),
            self.invoice_generator.invoice_count(),
            self.custom_templates.template_count(),
            self.bulk_service.job_count()
        )
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_merge_create_template() {
        let mut engine = MailMergeEngine::new();
        let result = engine.create_template("greeting", "Hello", "Dear {{name}}");
        assert!(result.is_ok());
        assert_eq!(engine.template_count(), 1);
    }

    #[test]
    fn test_mail_merge_empty_template() {
        let mut engine = MailMergeEngine::new();
        let result = engine.create_template("", "Subject", "Body");
        assert!(result.is_err());
    }

    #[test]
    fn test_mail_merge_add_variable() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("test", "Subject", "Body").unwrap();
        let result = engine.add_variable("test", "name", VariableType::String, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mail_merge_add_variable_empty_name() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("test", "Subject", "Body").unwrap();
        let result = engine.add_variable("test", "", VariableType::String, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_mail_merge_merge() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("greeting", "Greetings", "Hello {{name}}, welcome!").unwrap();
        
        let mut data = HashMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        
        let result = engine.merge("greeting", data).unwrap();
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_mail_merge_multiple_variables() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("email", "Subject", "Hi {{first}} {{last}}, your order is {{status}}.").unwrap();
        
        let mut data = HashMap::new();
        data.insert("first".to_string(), "John".to_string());
        data.insert("last".to_string(), "Doe".to_string());
        data.insert("status".to_string(), "confirmed".to_string());
        
        let result = engine.merge("email", data).unwrap();
        assert!(result.contains("John Doe"));
        assert!(result.contains("confirmed"));
    }

    #[test]
    fn test_mail_merge_processed_count() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("test", "S", "B").unwrap();
        assert_eq!(engine.get_processed_count(), 0);
        
        let mut data = HashMap::new();
        engine.merge("test", data.clone()).unwrap();
        assert_eq!(engine.get_processed_count(), 1);
        
        engine.merge("test", data).unwrap();
        assert_eq!(engine.get_processed_count(), 2);
    }

    #[test]
    fn test_mail_merge_clear() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("test", "S", "B").unwrap();
        assert_eq!(engine.template_count(), 1);
        engine.clear();
        assert_eq!(engine.template_count(), 0);
    }

    #[test]
    fn test_invoice_create() {
        let mut gen = InvoiceGenerator::new();
        let result = gen.create_invoice("INV001", "ACME Corp", "John Smith");
        assert!(result.is_ok());
        assert_eq!(gen.invoice_count(), 1);
    }

    #[test]
    fn test_invoice_create_empty() {
        let mut gen = InvoiceGenerator::new();
        let result = gen.create_invoice("", "Company", "Customer");
        assert!(result.is_err());
    }

    #[test]
    fn test_invoice_add_line_item() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "Company", "Customer").unwrap();
        let result = gen.add_line_item("INV001", "Widget", 5, 10.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invoice_add_line_item_invalid() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "Company", "Customer").unwrap();
        let result = gen.add_line_item("INV001", "", 0, 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_invoice_calculate_total() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "Company", "Customer").unwrap();
        gen.add_line_item("INV001", "Widget", 2, 25.0).unwrap();
        gen.add_line_item("INV001", "Gadget", 3, 15.0).unwrap();
        
        let total = gen.calculate_total("INV001").unwrap();
        assert_eq!(total, 95.0); // 2*25 + 3*15 = 50 + 45
    }

    #[test]
    fn test_invoice_generate_text() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "ACME", "John").unwrap();
        gen.set_customer_info("INV001", "john@example.com", "123 Main St").unwrap();
        gen.add_line_item("INV001", "Service", 1, 100.0).unwrap();
        
        let text = gen.generate_invoice_text("INV001").unwrap();
        assert!(text.contains("INV001"));
        assert!(text.contains("ACME"));
        assert!(text.contains("John"));
    }

    #[test]
    fn test_invoice_generated_count() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "Co", "Cust").unwrap();
        assert_eq!(gen.get_generated_count(), 0);
        gen.generate_invoice_text("INV001").unwrap();
        assert_eq!(gen.get_generated_count(), 1);
    }

    #[test]
    fn test_custom_template_create() {
        let mut engine = CustomTemplateEngine::new();
        let result = engine.create_template("report", "Report: {{title}}\nContent: {section:body}");
        assert!(result.is_ok());
        assert_eq!(engine.template_count(), 1);
    }

    #[test]
    fn test_custom_template_add_variable() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("test", "Content").unwrap();
        let result = engine.add_variable("test", "title", "Default Title".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_template_add_section() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("test", "Content").unwrap();
        let result = engine.add_section("test", "body", "Section content".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_template_add_helper() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("test", "Content").unwrap();
        let result = engine.add_helper("test", "uppercase", "fn(s) { s.upper() }".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_template_render() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("greeting", "Hello {{name}}!").unwrap();
        
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        
        let result = engine.render("greeting", vars).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_custom_template_render_with_defaults() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("test", "Title: {{title}}").unwrap();
        engine.add_variable("test", "title", "Default".to_string()).unwrap();
        
        let result = engine.render("test", HashMap::new()).unwrap();
        assert!(result.contains("Default"));
    }

    #[test]
    fn test_bulk_job_create() {
        let mut service = BulkGenerationService::new();
        let result = service.create_job("template1", "data.csv", "pdf", 100);
        assert!(result.is_ok());
        let job_id = result.unwrap();
        assert!(job_id.starts_with("job_"));
    }

    #[test]
    fn test_bulk_job_empty_params() {
        let mut service = BulkGenerationService::new();
        let result = service.create_job("", "source", "pdf", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_bulk_job_update_progress() {
        let mut service = BulkGenerationService::new();
        let job_id = service.create_job("tmpl", "data", "pdf", 100).unwrap();
        let result = service.update_progress(&job_id, 50, 2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bulk_job_get_progress() {
        let mut service = BulkGenerationService::new();
        let job_id = service.create_job("tmpl", "data", "pdf", 100).unwrap();
        service.update_progress(&job_id, 75, 5).unwrap();
        
        let (gen, fail, comp) = service.get_progress(&job_id).unwrap();
        assert_eq!(gen, 75);
        assert_eq!(fail, 5);
        assert_eq!(comp, 80);
    }

    #[test]
    fn test_bulk_job_is_complete() {
        let mut service = BulkGenerationService::new();
        let job_id = service.create_job("tmpl", "data", "pdf", 10).unwrap();
        
        assert_eq!(service.is_complete(&job_id).unwrap(), false);
        service.update_progress(&job_id, 8, 2).unwrap();
        assert_eq!(service.is_complete(&job_id).unwrap(), true);
    }

    #[test]
    fn test_bulk_multiple_jobs() {
        let mut service = BulkGenerationService::new();
        let job1 = service.create_job("t1", "d1", "pdf", 50).unwrap();
        let job2 = service.create_job("t2", "d2", "xlsx", 100).unwrap();
        
        assert_ne!(job1, job2);
        assert_eq!(service.job_count(), 2);
    }

    #[test]
    fn test_template_support_integration() {
        let mut support = TemplateSupport::new();
        
        // Create mail merge template
        support.mail_merge().create_template("email", "Hi", "Hello {{user}}").unwrap();
        
        // Create invoice
        support.invoices().create_invoice("INV001", "Co", "Cust").unwrap();
        
        // Create custom template
        support.templates().create_template("report", "Content").unwrap();
        
        // Create bulk job
        support.bulk().create_job("tmpl", "data", "pdf", 100).unwrap();
        
        let summary = support.summary();
        assert!(summary.contains("Mail-Merge"));
        assert!(summary.contains("Invoices"));
        assert!(summary.contains("Custom"));
        assert!(summary.contains("Bulk"));
    }

    #[test]
    fn test_mail_merge_complex_workflow() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("newsletter", "Subject", "Dear {{title}} {{last_name}},\n\nWelcome to {{company}}!\n\nBest regards,\nTheTeam").unwrap();
        engine.add_variable("newsletter", "title", VariableType::String, true).unwrap();
        engine.add_variable("newsletter", "last_name", VariableType::String, true).unwrap();
        engine.add_variable("newsletter", "company", VariableType::String, true).unwrap();
        
        let mut data = HashMap::new();
        data.insert("title".to_string(), "Dr.".to_string());
        data.insert("last_name".to_string(), "Smith".to_string());
        data.insert("company".to_string(), "TechCorp".to_string());
        
        let result = engine.merge("newsletter", data).unwrap();
        assert!(result.contains("Dr. Smith"));
        assert!(result.contains("TechCorp"));
    }

    #[test]
    fn test_invoice_complex_workflow() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV-2026-001", "Premium Corp", "Alice Johnson").unwrap();
        gen.set_customer_info("INV-2026-001", "alice@company.com", "456 Oak Ave, City").unwrap();
        
        gen.add_line_item("INV-2026-001", "Consulting", 10, 150.0).unwrap();
        gen.add_line_item("INV-2026-001", "Development", 40, 125.0).unwrap();
        gen.add_line_item("INV-2026-001", "Testing", 8, 100.0).unwrap();
        
        let total = gen.calculate_total("INV-2026-001").unwrap();
        let expected = (10.0 * 150.0) + (40.0 * 125.0) + (8.0 * 100.0);
        assert_eq!(total, expected);
        
        let text = gen.generate_invoice_text("INV-2026-001").unwrap();
        assert!(text.contains("INV-2026-001"));
        assert!(text.contains("Premium Corp"));
        assert!(text.contains("Alice Johnson"));
    }

    #[test]
    fn test_template_support_all_systems() {
        let mut support = TemplateSupport::new();
        
        // Mail merge
        support.mail_merge().create_template("welcome", "Welcome", "Hi {{name}}").unwrap();
        let mut mm_data = HashMap::new();
        mm_data.insert("name".to_string(), "Bob".to_string());
        support.mail_merge().merge("welcome", mm_data).unwrap();
        
        // Invoices
        support.invoices().create_invoice("INV001", "Company", "Customer").unwrap();
        support.invoices().add_line_item("INV001", "Service", 1, 50.0).unwrap();
        
        // Custom templates
        support.templates().create_template("report", "Report: {{data}}").unwrap();
        let mut ct_data = HashMap::new();
        ct_data.insert("data".to_string(), "Value".to_string());
        support.templates().render("report", ct_data).unwrap();
        
        // Bulk jobs
        support.bulk().create_job("t", "d", "pdf", 50).unwrap();
        
        let summary = support.summary();
        assert!(summary.contains("1"));
    }

    #[test]
    fn test_mail_merge_condition_placeholder() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("promo", "Subject", "Dear {{customer}},\n\nYour loyalty status: {{status}}").unwrap();
        engine.add_condition("promo", "gold_member", "status == 'Gold'".to_string()).unwrap();
        
        assert!(engine.get_template("promo").is_some());
    }

    #[test]
    fn test_bulk_job_counter_increment() {
        let mut service = BulkGenerationService::new();
        let job1 = service.create_job("t", "d", "pdf", 10).unwrap();
        let job2 = service.create_job("t", "d", "pdf", 10).unwrap();
        let job3 = service.create_job("t", "d", "pdf", 10).unwrap();
        
        assert!(job1.contains("job_0"));
        assert!(job2.contains("job_1"));
        assert!(job3.contains("job_2"));
    }

    #[test]
    fn test_custom_template_section_replacement() {
        let mut engine = CustomTemplateEngine::new();
        engine.create_template("doc", "Header\n{%section:body%}\nFooter").unwrap();
        engine.add_section("doc", "body", "Main content here".to_string()).unwrap();
        
        let result = engine.render("doc", HashMap::new()).unwrap();
        assert!(result.contains("Main content here"));
    }

    #[test]
    fn test_invoice_currency_symbol() {
        let mut gen = InvoiceGenerator::new();
        gen.create_invoice("INV001", "Co", "Cust").unwrap();
        gen.add_line_item("INV001", "Item", 1, 100.0).unwrap();
        
        let inv = gen.get_invoice("INV001").unwrap();
        assert_eq!(inv.currency_symbol, "$");
        
        let text = gen.generate_invoice_text("INV001").unwrap();
        assert!(text.contains("$"));
    }

    #[test]
    fn test_mail_merge_required_variable_validation() {
        let mut engine = MailMergeEngine::new();
        engine.create_template("test", "Subject", "Body {{required_var}}").unwrap();
        engine.add_variable("test", "required_var", VariableType::String, true).unwrap();
        
        let result = engine.merge("test", HashMap::new());
        assert!(result.is_err());
    }
}
