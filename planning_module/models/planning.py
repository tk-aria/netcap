from odoo import models, fields, api
from datetime import datetime, timedelta


class PlanningRole(models.Model):
    _name = 'planning.role'
    _description = 'Planning Role'
    _order = 'name'

    name = fields.Char('役割名', required=True)
    color = fields.Integer('Color')


class PlanningSlot(models.Model):
    _name = 'planning.slot'
    _description = 'Planning Slot'
    _order = 'start_datetime'
    _inherit = ['mail.thread']

    name = fields.Char('件名', compute='_compute_name', store=True)
    employee_id = fields.Many2one('hr.employee', string='担当者', required=True, tracking=True)
    role_id = fields.Many2one('planning.role', string='役割', tracking=True)
    department_id = fields.Many2one('hr.department', string='部門',
                                     related='employee_id.department_id', store=True)
    company_id = fields.Many2one('res.company', string='会社',
                                  default=lambda self: self.env.company)

    start_datetime = fields.Datetime('開始', required=True,
                                      default=lambda self: fields.Datetime.now().replace(hour=9, minute=0, second=0))
    end_datetime = fields.Datetime('終了', required=True,
                                    default=lambda self: fields.Datetime.now().replace(hour=18, minute=0, second=0))
    allocated_hours = fields.Float('予定時間', compute='_compute_allocated_hours', store=True)
    allocated_percentage = fields.Float('稼働率 %', default=100.0)

    state = fields.Selection([
        ('draft', '下書き'),
        ('published', '公開済'),
        ('done', '完了'),
    ], string='ステータス', default='draft', tracking=True)

    note = fields.Text('メモ')
    color = fields.Integer('Color', related='role_id.color')

    # Repeat
    repeat = fields.Boolean('繰り返し')
    repeat_type = fields.Selection([
        ('daily', '毎日'),
        ('weekly', '毎週'),
        ('monthly', '毎月'),
    ], string='繰り返しタイプ', default='weekly')
    repeat_until = fields.Date('繰り返し終了日')

    @api.depends('employee_id', 'role_id', 'start_datetime')
    def _compute_name(self):
        for slot in self:
            parts = []
            if slot.employee_id:
                parts.append(slot.employee_id.name)
            if slot.role_id:
                parts.append(slot.role_id.name)
            if slot.start_datetime:
                parts.append(slot.start_datetime.strftime('%m/%d'))
            slot.name = ' - '.join(parts) if parts else 'New Slot'

    @api.depends('start_datetime', 'end_datetime')
    def _compute_allocated_hours(self):
        for slot in self:
            if slot.start_datetime and slot.end_datetime:
                delta = slot.end_datetime - slot.start_datetime
                slot.allocated_hours = delta.total_seconds() / 3600.0
            else:
                slot.allocated_hours = 0.0

    def action_publish(self):
        self.write({'state': 'published'})

    def action_done(self):
        self.write({'state': 'done'})

    def action_draft(self):
        self.write({'state': 'draft'})

    def action_generate_repeats(self):
        """Generate repeated slots based on repeat settings."""
        new_slots = self.env['planning.slot']
        for slot in self:
            if not slot.repeat or not slot.repeat_until:
                continue
            current_start = slot.start_datetime
            current_end = slot.end_datetime
            duration = current_end - current_start

            while True:
                if slot.repeat_type == 'daily':
                    current_start += timedelta(days=1)
                elif slot.repeat_type == 'weekly':
                    current_start += timedelta(weeks=1)
                elif slot.repeat_type == 'monthly':
                    # Add ~30 days
                    month = current_start.month + 1
                    year = current_start.year
                    if month > 12:
                        month = 1
                        year += 1
                    current_start = current_start.replace(year=year, month=month)

                current_end = current_start + duration

                if current_start.date() > slot.repeat_until:
                    break

                new_slot = self.env['planning.slot'].create({
                    'employee_id': slot.employee_id.id,
                    'role_id': slot.role_id.id if slot.role_id else False,
                    'start_datetime': current_start,
                    'end_datetime': current_end,
                    'allocated_percentage': slot.allocated_percentage,
                    'note': slot.note,
                    'state': slot.state,
                })
                new_slots |= new_slot

        if new_slots:
            return {
                'type': 'ir.actions.act_window',
                'name': '生成されたスロット',
                'res_model': 'planning.slot',
                'view_mode': 'list,form,calendar',
                'domain': [('id', 'in', new_slots.ids)],
            }
        return True
