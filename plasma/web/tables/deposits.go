package tables

import (
	"fmt"

	"github.com/DryginAlexander/OpenPlasma/plasma/settings"

	"github.com/GoAdminGroup/go-admin/context"
	"github.com/GoAdminGroup/go-admin/modules/db"
	form2 "github.com/GoAdminGroup/go-admin/plugins/admin/modules/form"
	"github.com/GoAdminGroup/go-admin/plugins/admin/modules/table"
	"github.com/GoAdminGroup/go-admin/template/types"
	"github.com/GoAdminGroup/go-admin/template/types/form"
	editType "github.com/GoAdminGroup/go-admin/template/types/table"
)

func GetDepositTable(ctx *context.Context) (userTable table.Table) {

	userTable = table.NewDefaultTable(table.Config{
		Driver:     settings.DBDialect,
		CanAdd:     true,
		Editable:   true,
		Deletable:  true,
		Exportable: true,
		Connection: table.DefaultConnectionName,
		PrimaryKey: table.PrimaryKey{
			Type: db.Int,
			Name: table.DefaultPrimaryKeyName,
		},
	})

	info := userTable.GetInfo().SetFilterFormLayout(form.LayoutThreeCol)
	info.AddField("ID", "id", db.Int).FieldSortable()
	info.AddField("UserId", "user_id", db.Int).FieldSortable()
	info.AddField("Value", "value", db.Int).FieldSortable()
	info.AddField("CreatedAt", "created_at", db.Timestamp).
		FieldFilterable(types.FilterType{FormType: form.DatetimeRange})
	info.AddField("UpdatedAt", "updated_at", db.Timestamp).FieldEditAble(editType.Datetime)

	info.SetTable("deposits").SetTitle("Deposits").SetDescription("Deposits")

	formList := userTable.GetForm()
	formList.AddField("ID", "id", db.Int, form.Default).FieldNotAllowEdit().FieldNotAllowAdd()
	formList.AddField("UserId", "user_id", db.Int, form.Number)
	formList.AddField("Value", "value", db.Int, form.Number)
	formList.AddField("UpdatedAt", "updated_at", db.Timestamp, form.Default).FieldNotAllowAdd()
	formList.AddField("CreatedAt", "created_at", db.Timestamp, form.Default).FieldNotAllowAdd()

	formList.SetTable("deposits").SetTitle("Deposits").SetDescription("Deposits")

	formList.SetPostHook(func(values form2.Values) error {
		fmt.Println("userTable.GetForm().PostHook", values)
		return nil
	})

	return
}
